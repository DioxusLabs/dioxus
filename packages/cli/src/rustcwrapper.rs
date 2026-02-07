use serde::{Deserialize, Serialize};
use std::{
    env::{args, vars},
    path::PathBuf,
    process::ExitCode,
};

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

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RustcArgs {
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
    /// it doesn't include first program name argument
    pub link_args: Vec<String>,
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
                    let binary_u16le: Vec<u16> = file_binary
                        .chunks_exact(2)
                        .map(|a| u16::from_le_bytes([a[0], a[1]]))
                        .collect();
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

/// Run rustc directly, but output the result to a per-crate file in the args directory.
///
/// <https://doc.rust-lang.org/cargo/reference/config.html#buildrustc>
pub fn run_rustc() -> ExitCode {
    // If we are being asked to link, delegate to the linker action.
    if has_linking_args() {
        return crate::link::LinkAction::from_env()
            .expect("Linker action not found")
            .run_link();
    }

    let args_dir: PathBuf = std::env::var(DX_RUSTC_WRAPPER_ENV_VAR)
        .expect("DX_RUSTC env var must be set")
        .into();

    // Cargo invokes a workspace wrapper like: `wrapper-name rustc [args...]`
    // We skip our own executable name (`wrapper-name`) to get the args passed to us.
    let captured_args = args().skip(1).collect::<Vec<_>>();

    let rustc_args = RustcArgs {
        args: captured_args.clone(),
        envs: vars().collect::<_>(),
        link_args: Default::default(),
    };

    // Extract the crate name from the args to use as the filename.
    // Skip non-sensical args when a build is completely fresh (rustc is invoked with --crate-name ___)
    let crate_name = rustc_args
        .args
        .iter()
        .skip_while(|arg| *arg != "--crate-name")
        .nth(1);

    if let Some(crate_name) = crate_name {
        if crate_name != "___" {
            std::fs::create_dir_all(&args_dir)
                .expect("Failed to create args directory for rustc wrapper");

            let crate_type = rustc_args
                .args
                .iter()
                .skip_while(|arg| *arg != "--crate-type")
                .nth(1)
                .map(|s| s.as_str());

            let serialized_args =
                serde_json::to_string(&rustc_args).expect("Failed to serialize rustc args");

            // Always write to {crate_name}.json so dep crate args are found by name.
            // For crates with both lib and bin targets (src/lib.rs + src/main.rs),
            // cargo compiles lib first, then bin — the bin args overwrite the lib's.
            // To preserve the lib args, also write a copy to {crate_name}.lib.json.
            let main_file = args_dir.join(format!("{crate_name}.json"));
            let is_lib = matches!(crate_type, Some("lib" | "rlib"));

            if is_lib {
                let lib_file = args_dir.join(format!("{crate_name}.lib.json"));
                std::fs::write(&lib_file, &serialized_args)
                    .expect("Failed to write rustc lib args to file");
            }

            std::fs::write(&main_file, &serialized_args)
                .expect("Failed to write rustc args to file");

            eprintln!(
                "[dx-rustc-wrapper] Capturing args for crate '{}' (type={:?}) -> {}",
                crate_name,
                crate_type,
                main_file.display()
            );
        }
    }

    // Run the actual rustc command.
    // We want all stdout/stderr to be inherited, so the user sees the compiler output.
    let mut cmd = std::process::Command::new("rustc");

    // The first argument in `captured_args` is the rustc path, which we need to skip
    // when passing arguments to the `rustc` command we are spawning.
    cmd.args(captured_args.iter().skip(1));
    cmd.envs(rustc_args.envs);
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());
    cmd.current_dir(std::env::current_dir().expect("Failed to get current dir"));

    // Spawn the process and propagate its exit code.
    let status = cmd.status().expect("Failed to execute rustc command");
    std::process::exit(status.code().unwrap_or(1)); // Exit with 1 if process was killed by signal
}
