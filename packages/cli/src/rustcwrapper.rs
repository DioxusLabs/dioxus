use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env::{args, vars},
    path::PathBuf,
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

    /// Was a rustc invocation captured for `crate_name` (as a lib or bin) during this build?
    ///
    /// Workspace dependency graphs are target-agnostic, so a crate can be a workspace dependent
    /// of another without ever being part of *this* build's target/crate graph (e.g. a
    /// native-only sibling crate that a wasm32 build never compiles). This is used to filter such
    /// crates out before treating them as part of the current build.
    pub fn contains_crate(&self, crate_name: &str) -> bool {
        self.rustc_args.contains_key(&format!("{crate_name}.lib"))
            || self.rustc_args.contains_key(&format!("{crate_name}.bin"))
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

fn write_rustc_args(args_dir: &PathBuf, rustc_args: &RustcArgs) {
    // Extract the crate name from the args to use as the filename.
    // Skip non-sensical args when a build is completely fresh (rustc is invoked with --crate-name ___)
    let crate_name = rustc_args
        .args
        .iter()
        .skip_while(|arg| *arg != "--crate-name")
        .nth(1);

    if let Some(crate_name) = crate_name {
        if crate_name != "___" {
            std::fs::create_dir_all(args_dir)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args_for(names: &[&str]) -> WorkspaceRustcArgs {
        let mut workspace_args = WorkspaceRustcArgs::new(Vec::new());
        for name in names {
            workspace_args
                .rustc_args
                .insert(name.to_string(), RustcArgs::default());
        }
        workspace_args
    }

    #[test]
    fn contains_crate_matches_lib_key() {
        let workspace_args = args_for(&["shared_lib.lib"]);
        assert!(workspace_args.contains_crate("shared_lib"));
    }

    #[test]
    fn contains_crate_matches_bin_key() {
        let workspace_args = args_for(&["wasm_app.bin"]);
        assert!(workspace_args.contains_crate("wasm_app"));
    }

    #[test]
    fn contains_crate_false_for_uncaptured_crate() {
        // Mirrors the hot-patch cascade bug: a workspace dependent (e.g. a native-only sibling
        // crate) that was never compiled for the active build's target has no captured rustc
        // invocation at all, so it must not be treated as part of the current build.
        let workspace_args = args_for(&["shared_lib.lib", "wasm_app.bin"]);
        assert!(!workspace_args.contains_crate("native_ffi"));
    }

    #[test]
    fn contains_crate_false_on_empty_map() {
        let workspace_args = WorkspaceRustcArgs::new(Vec::new());
        assert!(!workspace_args.contains_crate("anything"));
    }
}
