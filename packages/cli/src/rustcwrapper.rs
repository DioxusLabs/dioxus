use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env::{args, vars},
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

/// Run rustc directly, but output the result to a per-crate file in the args directory.
///
/// <https://doc.rust-lang.org/cargo/reference/config.html#buildrustc>
pub fn run_rustc() -> ExitCode {
    let args_dir: PathBuf = std::env::var(DX_RUSTC_WRAPPER_ENV_VAR)
        .expect("DX_RUSTC env var must be set")
        .into();

    // Cargo invokes a workspace wrapper like: `wrapper-name rustc [args...]`
    // We skip our own executable name (`wrapper-name`) to get the args passed to us.
    let captured_args = args().skip(1).collect::<Vec<_>>();

    let rustc_args = RustcArgs {
        args: captured_args.clone(),
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

    // The first argument in `captured_args` is the rustc path, which we need to skip
    // when passing arguments to the `rustc` command we are spawning.
    cmd.args(captured_args.iter().skip(1));
    cmd.envs(rustc_args.envs);
    cmd.current_dir(rustc_args.cwd);
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    // Spawn the process and propagate its exit code.
    let status = cmd.status().expect("Failed to execute rustc command");
    std::process::exit(status.code().unwrap_or(1)); // Exit with 1 if process was killed by signal
}

fn write_rustc_args(args_dir: &Path, rustc_args: &RustcArgs) {
    // Extract the crate name from the args to use as the filename.
    // Skip non-sensical args when a build is completely fresh (rustc is invoked with --crate-name ___)
    let crate_name = rustc_args
        .args
        .iter()
        .skip_while(|arg| *arg != "--crate-name")
        .nth(1);

    if let Some(crate_name) = crate_name {
        if crate_name != "___" {
            create_private_dir_all(args_dir)
                .expect("Failed to create args directory for rustc wrapper");

            let crate_type = rustc_args
                .args
                .iter()
                .skip_while(|arg| *arg != "--crate-type")
                .nth(1)
                .map(|s| s.as_str());

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

            write_private_file(
                &args_dir.join(format!("{crate_name}.{suffix}.json")),
                &serialized_args,
            )
            .expect("Failed to write rustc args to file");
        }
    }
}

/// Create `dir` and any missing parents, then restrict `dir` itself to its owner.
#[cfg(unix)]
pub(crate) fn create_private_dir_all(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
pub(crate) fn create_private_dir_all(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

/// Write `contents` to `path`, creating or truncating it, and restrict it to its owner.
///
/// `mode` only applies when the file is created, so an existing file keeps its old mode
/// after truncation and needs `set_permissions` too.
#[cfg(unix)]
pub(crate) fn write_private_file(path: &Path, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    use std::{
        io::Write,
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(contents.as_ref())
}

#[cfg(not(unix))]
pub(crate) fn write_private_file(path: &Path, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    std::fs::write(path, contents)
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn write_private_file_tightens_existing_0644_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("probe.bin.json");
        std::fs::write(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        write_private_file(&path, "new").unwrap();

        assert_eq!(mode(&path), 0o600);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn write_private_file_creates_empty_file_as_0600() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("link_args.json");

        write_private_file(&path, "").unwrap();

        assert_eq!(mode(&path), 0o600);
        assert_eq!(std::fs::read(&path).unwrap(), b"");
    }

    #[test]
    fn write_private_file_errors_when_parent_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing").join("link_args.json");

        assert!(write_private_file(&path, "").is_err());
    }

    #[test]
    fn create_private_dir_all_tightens_existing_0755_dir() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("scope");
        std::fs::create_dir(&existing).unwrap();
        std::fs::set_permissions(&existing, std::fs::Permissions::from_mode(0o755)).unwrap();

        create_private_dir_all(&existing).unwrap();

        assert_eq!(mode(&existing), 0o700);
    }

    #[test]
    fn create_private_dir_all_creates_nested_dir_as_0700() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join(".captured-args").join("scope");

        create_private_dir_all(&nested).unwrap();

        assert_eq!(mode(&nested), 0o700);
    }
}
