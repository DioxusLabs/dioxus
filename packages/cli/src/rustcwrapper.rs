use serde::{Deserialize, Serialize};
use std::{
    env::{args, vars},
    path::PathBuf,
};

/// The environment variable indicating where the args file is located.
///
/// When `dx-rustc` runs, it writes its arguments to this file.
pub const DX_RUSTC_WRAPPER_ENV_VAR: &str = "DX_RUSTC";

/// Is `dx` being used as a rustc wrapper?
///
/// This is primarily used to intercept cargo, enabling fast hot-patching by caching the environment
/// cargo setups up for the user's current project.
///
/// In a differenet world we could simply rely on cargo printing link args and the rustc command, but
/// it doesn't seem to output that in a reliable, parseable, cross-platform format (ie using command
/// files on windows...), so we're forced to do this interception nonsense.
pub fn is_wrapping_rustc() -> bool {
    std::env::var(DX_RUSTC_WRAPPER_ENV_VAR).is_ok()
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RustcArgs {
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
    pub link_args: Vec<String>,
}

/// Run rustc directly, but output the result to a file.
///
/// <https://doc.rust-lang.org/cargo/reference/config.html#buildrustc>
pub async fn run_rustc() {
    // if we happen to be both a rustc wrapper and a linker, we want to run the linker if the arguments seem linker-y
    // this is a stupid hack
    if std::env::args()
        .take(5)
        .any(|arg| arg.ends_with(".o") || arg == "-flavor" || arg.starts_with("@"))
    {
        return crate::link::LinkAction::from_env()
            .expect("Linker action not found")
            .run_link()
            .await;
    }

    let var_file: PathBuf = std::env::var(DX_RUSTC_WRAPPER_ENV_VAR)
        .expect("DX_RUSTC not set")
        .into();

    let rustc_args = RustcArgs {
        args: args().skip(1).collect::<Vec<_>>(),
        envs: vars().collect::<_>(),
        link_args: Default::default(),
    };

    std::fs::create_dir_all(var_file.parent().expect("Failed to get parent dir"))
        .expect("Failed to create parent dir");
    std::fs::write(
        &var_file,
        serde_json::to_string(&rustc_args).expect("Failed to serialize rustc args"),
    )
    .expect("Failed to write rustc args to file");

    // Run the actual rustc command
    // We want all stdout/stderr to be inherited, so the running process can see the output
    //
    // Note that the args format we get from the wrapper includes the `rustc` command itself, so we
    // need to skip that - we already skipped the first arg when we created the args struct.
    let mut cmd = std::process::Command::new("rustc");
    cmd.args(rustc_args.args.iter().skip(1));
    cmd.envs(rustc_args.envs);
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());
    cmd.current_dir(std::env::current_dir().expect("Failed to get current dir"));

    // Propagate the exit code
    std::process::exit(cmd.status().unwrap().code().unwrap())
}
