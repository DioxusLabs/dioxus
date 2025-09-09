use std::{borrow::Cow, ffi::OsString};

use super::*;
use crate::{BuildMode, Result};
use anyhow::Context;

/// Perform a system analysis to verify the system install is working correctly.
#[derive(Clone, Debug, Subcommand)]
pub(crate) enum Print {
    /// Print the cargo args dioxus uses to build the server app.
    /// Environment variables will be set with the `env` command.
    #[clap(name = "client-args")]
    ClientArgs(PrintCargoArgs),

    /// Print the cargo args dioxus uses to build the client app.
    /// Environment variables will be set with the `env` command.
    #[clap(name = "server-args")]
    ServerArgs(PrintCargoArgs),
}

#[derive(Clone, Debug, Parser)]
pub(crate) struct PrintCargoArgs {
    #[clap(flatten)]
    pub(crate) args: CommandWithPlatformOverrides<build::BuildArgs>,

    /// The print output style to use. By default, this uses the current-platform's best fit,
    /// though you can customize it in the case you might be driving an external build system.
    /// - Unix style uses the `env` command
    /// - Windows style uses the `set` command
    /// - JSON style prints the arguments as JSON
    /// - Pretty JSON style prints the arguments as pretty JSON
    /// - Args style prints only the arguments, one per line, without any environment variables.
    /// - Env style prints only the environment variables, one key-pair per line, without any arguments.
    #[clap(long)]
    pub(crate) style: Option<PrintStyle>,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub(crate) enum PrintStyle {
    /// Print the arguments as a list of arguments, one per line.
    /// Does not include the `cargo rustc` command itself
    Args,

    /// Print the environment variables as a list of key=value pairs, one per line.
    Env,

    /// Print the arguments using the Unix `env` command.
    Unix,

    /// Print the arguments using the Windows `set` command.
    Cmd,

    /// Print the arguments as JSON. Does not include the `cargo rustc` command itself
    Json,

    /// Print the arguments as pretty JSON. Does not include the `cargo rustc` command itself
    PrettyJson,
}

impl Print {
    pub(crate) async fn print(self) -> Result<StructuredOutput> {
        match self {
            Self::ClientArgs(opts) => {
                let targets = opts.args.into_targets().await?;
                let mode = BuildMode::Base { run: false };
                let args = targets.client.cargo_build_arguments(&mode);
                let env = targets.client.cargo_build_env_vars(&mode)?;
                Self::print_as_unified_command(&env, &args, &opts.style);
                Ok(StructuredOutput::PrintCargoArgs {
                    args,
                    env: env
                        .into_iter()
                        .map(|(k, v)| (k, v.to_string_lossy().to_string()))
                        .collect::<Vec<_>>(),
                })
            }
            Self::ServerArgs(print_cargo_args) => {
                let targets = print_cargo_args.args.into_targets().await?;
                let mode = BuildMode::Base { run: false };
                let server = targets
                    .server
                    .context("No server target found, cannot print server args")?;
                let args = server.cargo_build_arguments(&mode);
                let env = server.cargo_build_env_vars(&mode)?;
                Self::print_as_unified_command(&env, &args, &print_cargo_args.style);
                Ok(StructuredOutput::PrintCargoArgs {
                    args,
                    env: env
                        .into_iter()
                        .map(|(k, v)| (k, v.to_string_lossy().to_string()))
                        .collect::<Vec<_>>(),
                })
            }
        }
    }

    /// Prints the given env and args as a unified command.
    /// - Uses `env` on unix systems
    /// - Uses `set VAR=value &&` on windows systems
    /// - Prints structured JSON on json style
    fn print_as_unified_command(
        env: &[(Cow<'static, str>, OsString)],
        args: &[String],
        style: &Option<PrintStyle>,
    ) {
        let style = style.clone().unwrap_or({
            if cfg!(unix) || std::env::var("MSYSTEM").is_ok() || std::env::var("CYGWIN").is_ok() {
                PrintStyle::Unix
            } else {
                PrintStyle::Cmd
            }
        });

        match style {
            PrintStyle::Args => {
                for arg in args {
                    println!("{}", arg);
                }
            }

            PrintStyle::Env => {
                for (key, value) in env {
                    println!("{}={}", key, value.to_string_lossy());
                }
            }

            PrintStyle::Unix => {
                let mut cmd = String::from("env");
                for (key, value) in env {
                    cmd.push_str(&format!(
                        " {}={}",
                        key,
                        shell_words::quote(&value.to_string_lossy())
                    ));
                }
                cmd.push_str(" cargo rustc");
                for arg in args {
                    cmd.push_str(&format!(" {}", shell_words::quote(arg)));
                }
                println!("{}", cmd);
            }
            PrintStyle::Cmd => {
                let mut cmd = String::new();
                for (key, value) in env {
                    cmd.push_str(&format!(
                        "set {}={} && ",
                        key,
                        Self::escape_windows(value.to_string_lossy())
                    ));
                }
                cmd.push_str("cargo rustc");
                for arg in args {
                    cmd.push_str(&format!(
                        " {}",
                        Self::escape_windows(Cow::Borrowed(arg.as_str()))
                    ));
                }
                println!("{}", cmd);
            }
            PrintStyle::Json | PrintStyle::PrettyJson => {
                let output = serde_json::json!({
                    "env": env.iter().map(|(k, v)| (k.as_ref(), v)).collect::<std::collections::HashMap<_, _>>(),
                    "args": args
                });
                if matches!(style, PrintStyle::PrettyJson) {
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                } else {
                    println!("{}", serde_json::to_string(&output).unwrap());
                }
            }
        }
    }
    /// Escape for the windows cmd.exe shell.
    ///
    /// See [here][msdn] for more information.
    ///
    /// [msdn]: http://blogs.msdn.com/b/twistylittlepassagesallalike/archive/2011/04/23/everyone-quotes-arguments-the-wrong-way.aspx
    ///
    /// This function comes from shell-escape
    fn escape_windows(s: Cow<str>) -> Cow<str> {
        use std::iter::repeat;

        let mut needs_escape = s.is_empty();
        for ch in s.chars() {
            match ch {
                '"' | '\t' | '\n' | ' ' => needs_escape = true,
                _ => {}
            }
        }
        if !needs_escape {
            return s;
        }
        let mut es = String::with_capacity(s.len());
        es.push('"');
        let mut chars = s.chars().peekable();
        loop {
            let mut nslashes = 0;
            while let Some(&'\\') = chars.peek() {
                chars.next();
                nslashes += 1;
            }

            match chars.next() {
                Some('"') => {
                    es.extend(repeat('\\').take(nslashes * 2 + 1));
                    es.push('"');
                }
                Some(c) => {
                    es.extend(repeat('\\').take(nslashes));
                    es.push(c);
                }
                None => {
                    es.extend(repeat('\\').take(nslashes * 2));
                    break;
                }
            }
        }
        es.push('"');
        es.into()
    }
}
