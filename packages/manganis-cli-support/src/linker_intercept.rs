use std::{ffi::OsStr, fs, path::PathBuf, process::Stdio};

// The prefix to link args passed from parent process.
const MG_ARG_NAME: &str = "mg-arg=";

/// Intercept the linker for object files.
///
/// Takes the arguments used in a CLI and returns a list of paths to `.rlib` or `.o` files to be searched for asset sections.
pub fn linker_intercept<I, T>(args: I) -> Option<(Vec<String>, Vec<PathBuf>)>
where
    I: IntoIterator<Item = T>,
    T: ToString,
{
    let args: Vec<String> = args.into_iter().map(|x| x.to_string()).collect();

    // Check if we were provided with a command file.
    let mut is_command_file = None;
    for arg in args.iter() {
        // On windows the linker args are passed in a file that is referenced by `@<file>`
        if arg.starts_with('@') {
            is_command_file = Some(arg.clone());
            break;
        }
    }

    let linker_args = match is_command_file {
        // On unix/linux/mac the linker args are passed directly.
        None => args,
        // Handle windows here - uf16le and utf8 files are supported.
        Some(arg) => {
            let path = arg.trim().trim_start_matches('@');
            let file_binary = fs::read(path).unwrap();

            // This may be a utf-16le file. Let's try utf-8 first.
            let content = match String::from_utf8(file_binary.clone()) {
                Ok(s) => s,
                Err(_) => {
                    // Convert Vec<u8> to Vec<u16> to convert into a String
                    let binary_u16le: Vec<u16> = file_binary
                        .chunks_exact(2)
                        .map(|a| u16::from_le_bytes([a[0], a[1]]))
                        .collect();

                    String::from_utf16_lossy(&binary_u16le)
                }
            };

            // Gather linker args
            let mut linker_args = Vec::new();
            let lines = content.lines();

            for line in lines {
                // Remove quotes from the line - windows link args files are quoted
                let line_parsed = line.to_string();
                let line_parsed = line_parsed.trim_end_matches('"').to_string();
                let line_parsed = line_parsed.trim_start_matches('"').to_string();

                linker_args.push(line_parsed);
            }

            linker_args
        }
    };

    let mut link_args = Vec::new();

    // Parse through linker args for `.o` or `.rlib` files.
    let mut object_files: Vec<PathBuf> = Vec::new();
    for item in linker_args {
        // Get the working directory so it isn't lost.
        // When rust calls the linker it doesn't pass the working dir so we need to recover it.
        // "{MG_WORKDIR_ARG_NAME}path"
        if item.starts_with(MG_ARG_NAME) {
            let split: Vec<_> = item.split('=').collect();
            link_args.push(split[1].to_string());
            continue;
        }

        if item.ends_with(".o") || item.ends_with(".rlib") {
            object_files.push(PathBuf::from(item));
        }
    }

    if object_files.is_empty() {
        return None;
    }

    Some((link_args, object_files))
}

/// Calls cargo to build the project with a linker intercept script.
///
/// The linker intercept script will call the current executable with the specified subcommand
/// and a list of arguments provided by rustc.
pub fn start_linker_intercept<I, J>(
    subcommand: &str,
    args: I,
    link_args: Option<J>,
) -> Result<(), std::io::Error>
where
    I: IntoIterator,
    I::Item: AsRef<OsStr>,
    J: IntoIterator,
    J::Item: ToString,
{
    let exec_path = std::env::current_exe().unwrap();

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("rustc");
    cmd.args(args);
    cmd.arg("--");

    // Build a temporary redirect script.
    let script_path = create_linker_script(exec_path, subcommand).unwrap();
    let linker_arg = format!("-Clinker={}", script_path.display());
    cmd.arg(linker_arg);

    // Handle passing any arguments back to the current executable.
    if let Some(link_args) = link_args {
        let link_args: Vec<String> = link_args.into_iter().map(|x| x.to_string()).collect();
        for link_arg in link_args {
            let arg = format!("-Clink-arg={}{}", MG_ARG_NAME, link_arg);
            cmd.arg(arg);
        }
    }

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait()?;
    Ok(())
}

const LINK_SCRIPT_NAME: &str = "mg-link";

/// Creates a temporary script that re-routes rustc linker args to a subcommand of an executable.
fn create_linker_script(exec: PathBuf, subcommand: &str) -> Result<PathBuf, std::io::Error> {
    #[cfg(windows)]
    let (script, ext) = (
        format!("echo off\n{} {} %*", exec.display(), subcommand),
        "bat",
    );
    #[cfg(not(windows))]
    let (script, ext) = (
        format!("#!/usr/bin/env bash\n{} {} $@", exec.display(), subcommand),
        "sh",
    );

    let temp_path = std::env::temp_dir();
    let out_name = format!("{LINK_SCRIPT_NAME}.{ext}");
    let out = temp_path.join(out_name);
    fs::write(&out, script)?;

    // Set executable permissions.
    let mut perms = fs::metadata(&out)?.permissions();

    // We give windows RW and implied X perms.
    // Clippy complains on any platform about this even if it's not *nix.
    // https://rust-lang.github.io/rust-clippy/master/index.html#permissions_set_readonly_false
    #[cfg(windows)]
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);

    // We give nix user-RWX perms.
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o700);
    }
    fs::set_permissions(&out, perms)?;

    Ok(out)
}

/// Deletes the temporary script created by `create_linker_script`.
pub fn delete_linker_script() -> Result<(), std::io::Error> {
    #[cfg(windows)]
    let ext = "bat";
    #[cfg(not(windows))]
    let ext = "sh";

    let temp_path = std::env::temp_dir();
    let file_name = format!("{LINK_SCRIPT_NAME}.{ext}");
    let file = temp_path.join(file_name);
    fs::remove_file(file)
}
