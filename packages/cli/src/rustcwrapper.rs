use std::{collections::HashMap, path::PathBuf, process::ExitCode};

use crate::{Platform, Result};

/// The environment variable indicating where the args file is located.
///
/// When `dx-rustc` runs, it writes its arguments to this file.
pub const RUSTC_WRAPPER_ENV_VAR: &str = "DX_RUSTC";

pub fn is_rustc() -> bool {
    std::env::var(RUSTC_WRAPPER_ENV_VAR).is_ok()
}

#[derive(Default, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RustcArgs {
    pub args: Vec<String>,
    pub envs: HashMap<String, String>,
}

/// Run rustc directly, but output the result to a file.
///
/// https://doc.rust-lang.org/cargo/reference/config.html#buildrustc
pub async fn run_rustc() {
    let var_file: PathBuf = std::env::var(RUSTC_WRAPPER_ENV_VAR)
        .expect("DX_RUSTC not set")
        .into();

    let rustc_args = RustcArgs {
        envs: std::env::vars()
            .map(|(k, v)| (k, v))
            .collect::<HashMap<_, _>>(),
        args: std::env::args().skip(1).collect::<Vec<_>>(),
    };

    std::fs::create_dir_all(var_file.parent().expect("Failed to get parent dir"))
        .expect("Failed to create parent dir");
    std::fs::write(
        &var_file,
        serde_json::to_string(&rustc_args).expect("Failed to serialize rustc args"),
    )
    .expect("Failed to write rustc args to file");

    // Run the actual rustc command
    let mut cmd = std::process::Command::new("rustc");
    cmd.args(rustc_args.args.iter().skip(1));
    cmd.envs(rustc_args.envs);
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());
    cmd.current_dir(std::env::current_dir().expect("Failed to get current dir"));

    // Propagate the exit code
    std::process::exit(cmd.status().unwrap().code().unwrap())
}
