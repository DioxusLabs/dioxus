use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env::{args, vars},
    io::Read,
    path::{Path, PathBuf},
    process::ExitCode,
};

/// A "capture" of a workspace's rustc commands, cumulated by reading the various rustc commands
/// from disk.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkspaceRustcArgs {
    pub link_args: Vec<String>,
    pub rustc_args: HashMap<String, RustcArgs>,
}

impl WorkspaceRustcArgs {
    pub fn new(link_args: Vec<String>) -> Self {
        Self {
            link_args,
            rustc_args: Default::default(),
        }
    }

    /// The captured rustc invocation used to replay a workspace crate in a thin build.
    /// `None` means the fat build never compiled the crate.
    pub fn replay_args(&self, crate_name: &str) -> Option<&RustcArgs> {
        self.rustc_args
            .get(&format!("{crate_name}.lib"))
            .or_else(|| self.rustc_args.get(&format!("{crate_name}.bin")))
    }
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RustcArgs {
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
    pub cwd: PathBuf,
}

/// The environment variable indicating where the args directory is located.
///
/// When `dx-rustc` runs, it writes each workspace crate's arguments to a
/// separate file in this directory: `{dir}/{crate_name}.json`.
pub const DX_RUSTC_WRAPPER_ENV_VAR: &str = "DX_RUSTC";

/// Is `dx` being used as a rustc wrapper?
///
/// This is primarily used to intercept cargo, enabling fast hot-patching by caching the environment
/// cargo setups up for the user's current project.
///
/// In a different world we could simply rely on cargo printing link args and the rustc command, but
/// it doesn't seem to output that in a reliable, parseable, cross-platform format (ie using command
/// files on windows...), so we're forced to do this interception nonsense.
pub fn is_wrapping_rustc() -> bool {
    std::env::var(DX_RUSTC_WRAPPER_ENV_VAR).is_ok()
}

/// Maximum combined command-line length before spilling `rustc` arguments into an `@argfile`.
/// Kept well below Windows' 32,767-character `CreateProcessW` limit.
const RUSTC_ARGFILE_THRESHOLD: usize = 16_000;

/// Maximum size in bytes of a single `@<path>` rustc response file we will read and expand.
const MAX_ARGFILE_BYTES: usize = 8 * 1024 * 1024;

/// Maximum cumulative bytes read/expanded across all `@<path>` response files in one invocation.
const MAX_EXPANDED_ARGFILE_BYTES: usize = 16 * 1024 * 1024;

/// Maximum cumulative number of arguments expanded from `@<path>` response files in one invocation.
const MAX_EXPANDED_ARGFILE_ARGS: usize = 100_000;

fn read_bounded_argfile(path: &Path, max_bytes: usize) -> (usize, Option<String>) {
    // Reject missing paths, special files (devices, FIFOs, directories, etc.), and
    // oversized regular files before opening so we never block on a FIFO or open a device.
    let Ok(meta) = std::fs::metadata(path) else {
        return (0, None);
    };
    if !meta.is_file() || meta.len() > max_bytes as u64 {
        return (0, None);
    }

    let Ok(file) = std::fs::File::open(path) else {
        return (0, None);
    };
    let Ok(file_meta) = file.metadata() else {
        return (0, None);
    };
    if !file_meta.is_file() || file_meta.len() > max_bytes as u64 {
        return (0, None);
    }

    let initial_cap = (file_meta.len() as usize).min(max_bytes).min(64 * 1024);
    let mut buf = Vec::with_capacity(initial_cap);
    let read_res = file
        .take((max_bytes as u64).saturating_add(1))
        .read_to_end(&mut buf);
    let bytes_read = buf.len();
    if read_res.is_err() || bytes_read > max_bytes {
        return (bytes_read, None);
    }

    let Ok(content) = String::from_utf8(buf) else {
        return (bytes_read, None);
    };
    if content.contains('\0') {
        return (bytes_read, None);
    }

    (bytes_read, Some(content))
}

/// Expand any `@<path>` rustc response files in `raw_args[1..]` while preserving `raw_args[0]`.
///
/// Matches `rustc`'s newline-delimited, unquoted UTF-8 `@path` format (stripping an optional
/// leading UTF-8 BOM). Bare `"@"`, non-existent paths, non-regular/special files, non-UTF-8
/// files (such as UTF-16LE MSVC linker response files), and files exceeding the per-file or
/// cumulative expansion limits are left intact.
pub(crate) fn expand_rustc_args(raw_args: &[String]) -> Vec<String> {
    expand_rustc_args_with_limits(
        raw_args,
        MAX_ARGFILE_BYTES,
        MAX_EXPANDED_ARGFILE_BYTES,
        MAX_EXPANDED_ARGFILE_ARGS,
    )
}

fn expand_rustc_args_with_limits(
    raw_args: &[String],
    max_file_bytes: usize,
    max_total_bytes: usize,
    max_total_args: usize,
) -> Vec<String> {
    let mut expanded = Vec::with_capacity(raw_args.len());
    let Some((first, rest)) = raw_args.split_first() else {
        return expanded;
    };
    expanded.push(first.clone());

    let mut remaining_bytes = max_total_bytes;
    let mut remaining_args = max_total_args;

    for arg in rest {
        if let Some(path_str) = arg.strip_prefix('@') {
            let max_read = max_file_bytes.min(remaining_bytes);
            if !path_str.is_empty() && max_read > 0 && remaining_args > 0 {
                let (bytes_read, content) = read_bounded_argfile(Path::new(path_str), max_read);
                remaining_bytes = remaining_bytes.saturating_sub(bytes_read);
                if let Some(content) = content {
                    let stripped = content.strip_prefix('\u{feff}').unwrap_or(&content);
                    let line_count = stripped
                        .lines()
                        .take(remaining_args.saturating_add(1))
                        .count();
                    if line_count <= remaining_args {
                        remaining_args -= line_count;
                        expanded.reserve(line_count);
                        expanded.extend(stripped.lines().map(str::to_string));
                        continue;
                    }
                }
            }
        }
        expanded.push(arg.clone());
    }

    expanded
}

/// Prepare arguments for invoking `rustc` directly.
///
/// Returns `args` inline when the combined length is below [`RUSTC_ARGFILE_THRESHOLD`] or any
/// argument contains `\n`/`\r`. Otherwise writes `args` as unquoted UTF-8 lines to `argfile_path`
/// and returns `vec!["@<argfile_path>"]`.
pub(crate) fn prepare_rustc_command_args(
    args: &[String],
    argfile_path: &Path,
) -> std::io::Result<Vec<String>> {
    let total_len: usize = args.iter().map(|arg| arg.len() + 1).sum();
    let has_line_break = args
        .iter()
        .any(|arg| arg.contains('\n') || arg.contains('\r'));

    if total_len < RUSTC_ARGFILE_THRESHOLD || has_line_break {
        return Ok(args.to_vec());
    }

    if let Some(parent) = argfile_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    std::fs::write(argfile_path, args.join("\n"))?;
    Ok(vec![format!("@{}", argfile_path.display())])
}

/// Run rustc directly, but output the result to a per-crate file in the args directory.
///
/// <https://doc.rust-lang.org/cargo/reference/config.html#buildrustc>
pub fn run_rustc() -> ExitCode {
    let args_dir: PathBuf = std::env::var(DX_RUSTC_WRAPPER_ENV_VAR)
        .expect("DX_RUSTC env var must be set")
        .into();

    // Cargo invokes a workspace wrapper like: `wrapper-name rustc [args...]`
    // We skip our own executable name (`wrapper-name`) to get the args passed to us.
    let raw_args = args().skip(1).collect::<Vec<_>>();
    let expanded_args = expand_rustc_args(&raw_args);

    let rustc_args = RustcArgs {
        args: expanded_args,
        envs: vars().collect::<_>(),
        cwd: std::env::current_dir().expect("Failed to get current dir"),
    };

    // Always persist the captured rustc invocation, even for link steps.
    // The tip crate's bin target is typically only observed during the final link invocation,
    // so returning early before writing would lose the exact args/envs we need for fat-link replay.
    write_rustc_args(&args_dir, &rustc_args);

    // If we are being asked to link, delegate to the linker action after capturing.
    if has_linking_args() {
        return crate::link::LinkAction::from_env()
            .expect("Linker action not found")
            .run_link();
    }

    // Run the actual rustc command.
    // We want all stdout/stderr to be inherited, so the user sees the compiler output.
    let mut cmd = std::process::Command::new("rustc");

    // The first argument in `raw_args` is the rustc path, which we need to skip
    // when passing arguments to the `rustc` command we are spawning. Forward the raw
    // arguments so Cargo's still-live `@argfile` is preserved during the initial build.
    cmd.args(raw_args.iter().skip(1));
    cmd.envs(rustc_args.envs);
    cmd.current_dir(rustc_args.cwd);
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    // Spawn the process and propagate its exit code.
    let status = cmd.status().expect("Failed to execute rustc command");
    std::process::exit(status.code().unwrap_or(1)); // Exit with 1 if process was killed by signal
}

fn extract_flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    for (idx, arg) in args.iter().enumerate() {
        if arg == flag {
            return args.get(idx + 1).map(String::as_str);
        }
        if let Some(rest) = arg.strip_prefix(flag) {
            if let Some(value) = rest.strip_prefix('=') {
                return Some(value);
            }
        }
    }
    None
}

fn write_rustc_args(args_dir: &Path, rustc_args: &RustcArgs) {
    // Extract the crate name from the args to use as the filename.
    // Skip non-sensical args when a build is completely fresh (rustc is invoked with --crate-name ___)
    let crate_name = extract_flag_value(&rustc_args.args, "--crate-name");

    if let Some(crate_name) = crate_name {
        if !crate_name.is_empty() && crate_name != "___" {
            std::fs::create_dir_all(args_dir)
                .expect("Failed to create args directory for rustc wrapper");

            let crate_type = extract_flag_value(&rustc_args.args, "--crate-type");

            let serialized_args =
                serde_json::to_string(rustc_args).expect("Failed to serialize rustc args");

            // Write args with an explicit target suffix: {crate_name}.lib.json or
            // {crate_name}.bin.json. This avoids the ambiguity of a bare {crate_name}.json
            // and ensures lib+bin crates don't overwrite each other.
            let suffix = match crate_type {
                Some("lib" | "rlib") => "lib",
                Some("bin") => "bin",
                _ => "bin", // proc-macro, cdylib, etc. — treat as bin
            };

            std::fs::write(
                args_dir.join(format!("{crate_name}.{suffix}.json")),
                &serialized_args,
            )
            .expect("Failed to write rustc args to file");
        }
    }
}

/// Check if the arguments indicate a linking step, including those in command files.
fn has_linking_args() -> bool {
    for arg in std::env::args() {
        // Direct check for linker-like arguments
        if arg.ends_with(".o") || arg == "-flavor" {
            return true;
        }

        // Check inside command files
        if let Some(path_str) = arg.strip_prefix('@') {
            if let Ok(file_binary) = std::fs::read(path_str) {
                // Handle both UTF-8 and UTF-16LE encodings for response files.
                let content = String::from_utf8(file_binary.clone()).unwrap_or_else(|_| {
                    let (pairs, _) = file_binary.as_chunks::<2>();
                    let binary_u16le: Vec<u16> =
                        pairs.iter().map(|&p| u16::from_le_bytes(p)).collect();
                    String::from_utf16_lossy(&binary_u16le)
                });

                // Check if any line in the command file contains linking indicators.
                if content.lines().any(|line| {
                    let trimmed_line = line.trim().trim_matches('"');
                    trimmed_line.ends_with(".o") || trimmed_line == "-flavor"
                }) {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_and_write_argfile_captured_bin_and_lib_args() {
        let tmp = tempfile::tempdir().unwrap();
        let capture_dir = tmp.path().join("captured");

        // Regression: Cargo passes rustc args via @<argfile> when the command line is long.
        let bin_argfile = tmp.path().join("bin.rsp");
        std::fs::write(
            &bin_argfile,
            "--crate-name\nhotpatching_systems\n--crate-type\nbin\n-L\ndependency=/tmp/target/debug/deps\n",
        )
        .unwrap();

        let raw_bin = vec!["rustc".to_string(), format!("@{}", bin_argfile.display())];
        let expanded_bin = expand_rustc_args(&raw_bin);
        assert_eq!(
            expanded_bin,
            vec![
                "rustc",
                "--crate-name",
                "hotpatching_systems",
                "--crate-type",
                "bin",
                "-L",
                "dependency=/tmp/target/debug/deps",
            ]
        );

        let bin_rustc_args = RustcArgs {
            args: expanded_bin.clone(),
            envs: vec![],
            cwd: tmp.path().to_path_buf(),
        };
        write_rustc_args(&capture_dir, &bin_rustc_args);

        let bin_json = capture_dir.join("hotpatching_systems.bin.json");
        assert!(bin_json.exists(), "expected hotpatching_systems.bin.json");
        let persisted: RustcArgs =
            serde_json::from_str(&std::fs::read_to_string(&bin_json).unwrap()).unwrap();
        assert_eq!(persisted.args, expanded_bin);

        // Happy path: direct args for both lib and bin, plus = forms.
        let direct_lib = RustcArgs {
            args: vec![
                "rustc".into(),
                "--crate-name".into(),
                "my_lib".into(),
                "--crate-type".into(),
                "lib".into(),
            ],
            envs: vec![],
            cwd: tmp.path().to_path_buf(),
        };
        write_rustc_args(&capture_dir, &direct_lib);
        assert!(capture_dir.join("my_lib.lib.json").exists());

        let direct_bin = RustcArgs {
            args: vec![
                "rustc".into(),
                "--crate-name".into(),
                "my_bin".into(),
                "--crate-type".into(),
                "bin".into(),
            ],
            envs: vec![],
            cwd: tmp.path().to_path_buf(),
        };
        write_rustc_args(&capture_dir, &direct_bin);
        assert!(capture_dir.join("my_bin.bin.json").exists());

        let eq_argfile = tmp.path().join("eq.rsp");
        std::fs::write(&eq_argfile, "--crate-name=foo\n--crate-type=lib\n").unwrap();
        let eq_args = RustcArgs {
            args: expand_rustc_args(&["rustc".into(), format!("@{}", eq_argfile.display())]),
            envs: vec![],
            cwd: tmp.path().to_path_buf(),
        };
        write_rustc_args(&capture_dir, &eq_args);
        assert!(capture_dir.join("foo.lib.json").exists());
    }

    #[test]
    fn expand_rustc_args_handles_bom_crlf_spaces_and_multiple_argfiles() {
        let tmp = tempfile::tempdir().unwrap();
        let first = tmp.path().join("first.rsp");
        let second = tmp.path().join("second.rsp");

        // UTF-8 BOM + CRLF + unquoted path containing spaces.
        std::fs::write(
            &first,
            "\u{feff}--crate-name\r\nmy_app\r\n-L\r\ndependency=C:\\Program Files\\My App\\deps\r\n",
        )
        .unwrap();
        std::fs::write(&second, "--crate-type\nbin\n--edition=2024\n").unwrap();

        let first_arg = format!("@{}", first.display());
        let raw = vec![
            first_arg.clone(), // args[0] must be preserved, not expanded
            "--cfg".into(),
            "debug_assertions".into(),
            first_arg.clone(),
            "-O".into(),
            format!("@{}", second.display()),
        ];

        assert_eq!(
            expand_rustc_args(&raw),
            vec![
                first_arg.as_str(),
                "--cfg",
                "debug_assertions",
                "--crate-name",
                "my_app",
                "-L",
                "dependency=C:\\Program Files\\My App\\deps",
                "-O",
                "--crate-type",
                "bin",
                "--edition=2024",
            ]
        );
    }

    #[test]
    fn expand_rustc_args_error_paths_and_probe_crate_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("does_not_exist.rsp");
        let utf16_msvc = tmp.path().join("msvc_linker.rsp");

        // UTF-16LE with BOM (invalid UTF-8), matching MSVC linker response files.
        let mut bytes = vec![0xFF, 0xFE];
        for code_unit in "\"foo.o\" /OUT:app.exe".encode_utf16() {
            bytes.extend_from_slice(&code_unit.to_le_bytes());
        }
        std::fs::write(&utf16_msvc, bytes).unwrap();

        let missing_arg = format!("@{}", missing.display());
        let utf16_arg = format!("@{}", utf16_msvc.display());
        let raw = vec![
            "rustc".into(),
            "@".into(),
            missing_arg.clone(),
            utf16_arg.clone(),
            "--crate-name".into(),
            "___".into(),
        ];

        let expanded = expand_rustc_args(&raw);
        assert_eq!(expanded, raw);

        // `___` probe invocations must still be skipped.
        let capture_dir = tmp.path().join("captured");
        write_rustc_args(
            &capture_dir,
            &RustcArgs {
                args: expanded,
                envs: vec![],
                cwd: tmp.path().to_path_buf(),
            },
        );
        assert!(!capture_dir.exists());
    }

    #[test]
    fn prepare_rustc_command_args_spills_only_when_needed() {
        let tmp = tempfile::tempdir().unwrap();

        // Below threshold: returned inline without creating an argfile.
        let short_args = vec![
            "--crate-name".into(),
            "demo".into(),
            "--crate-type".into(),
            "bin".into(),
        ];
        let short_rsp = tmp.path().join("unused/short.rsp");
        let prepared = prepare_rustc_command_args(&short_args, &short_rsp).unwrap();
        assert_eq!(prepared, short_args);
        assert!(!short_rsp.exists());

        // Above threshold: written as unquoted newline-delimited UTF-8 and passed as @<path>.
        let long_args: Vec<String> = (0..400)
            .map(|i| format!("--extern=crate_{i}=/path with spaces/libcrate_{i}.rlib"))
            .collect();
        let long_rsp = tmp.path().join("nested/long.rsp");
        let prepared = prepare_rustc_command_args(&long_args, &long_rsp).unwrap();
        assert_eq!(prepared, vec![format!("@{}", long_rsp.display())]);
        assert_eq!(
            std::fs::read_to_string(&long_rsp).unwrap(),
            long_args.join("\n")
        );

        // Embedded newline/CR falls back to inline even when over the threshold.
        let mut newline_args = long_args;
        newline_args.push("has\nnewline".into());
        let newline_rsp = tmp.path().join("nested/newline.rsp");
        let prepared = prepare_rustc_command_args(&newline_args, &newline_rsp).unwrap();
        assert_eq!(prepared, newline_args);
        assert!(!newline_rsp.exists());
    }

    #[test]
    fn expand_rustc_args_enforces_per_file_cumulative_and_special_file_limits() {
        let tmp = tempfile::tempdir().unwrap();

        let exact_file = tmp.path().join("exact.rsp");
        std::fs::write(&exact_file, "a\nb\n").unwrap(); // 4 bytes, 2 args

        let too_large = tmp.path().join("too_large.rsp");
        std::fs::write(&too_large, "12345").unwrap(); // 5 bytes (> max_file_bytes = 4)

        let dir_path = tmp.path().join("subdir");
        std::fs::create_dir(&dir_path).unwrap();

        let exact_arg = format!("@{}", exact_file.display());
        let too_large_arg = format!("@{}", too_large.display());
        let dir_arg = format!("@{}", dir_path.display());

        // Oversized file and directory are left intact without consuming cumulative budget;
        // repeated valid @<path> expands only until the cumulative byte limit is reached.
        let raw = vec![
            "rustc".into(),
            too_large_arg.clone(),
            dir_arg.clone(),
            exact_arg.clone(),
            exact_arg.clone(),
            exact_arg.clone(),
        ];
        assert_eq!(
            expand_rustc_args_with_limits(&raw, 4, 8, 100),
            vec![
                "rustc",
                too_large_arg.as_str(),
                dir_arg.as_str(),
                "a",
                "b",
                "a",
                "b",
                exact_arg.as_str(),
            ]
        );

        // Cumulative argument count cap leaves overflowing files intact rather than truncating.
        let raw_arg_cap = vec![
            "rustc".into(),
            exact_arg.clone(),
            exact_arg.clone(),
        ];
        assert_eq!(
            expand_rustc_args_with_limits(&raw_arg_cap, 1024, 1024, 3),
            vec!["rustc", "a", "b", exact_arg.as_str()]
        );

        #[cfg(unix)]
        {
            let dev_zero_symlink = tmp.path().join("zero_link.rsp");
            std::os::unix::fs::symlink("/dev/zero", &dev_zero_symlink).unwrap();
            let raw_special = vec![
                "rustc".into(),
                "@/dev/zero".into(),
                format!("@{}", dev_zero_symlink.display()),
            ];
            assert_eq!(expand_rustc_args(&raw_special), raw_special);
        }
    }
}
