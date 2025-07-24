use serde_json::json;

use super::*;
use crate::{telemetry::Anonymized, AddressArguments, BuildArgs, TraceController};
/// Serve the project
///
/// `dx serve` takes cargo args by default with additional renderer args like `--web`, `--webview`, and `--native`:
///
/// ```sh
/// dx serve --example blah --target blah --android
/// ```
///
/// A simple serve:
/// ```sh
/// dx serve --web
/// ```
///
/// As of dioxus 0.7, `dx serve` allows independent customization of the client and server builds,
/// allowing workspaces and removing any "magic" done to support ergonomic fullstack serving with
/// an plain `dx serve`. These require specifying more arguments like features since they won't be autodetected.
///
/// ```sh
/// dx serve \
///     client --package frontend \
///     server --package backend
/// ```
#[derive(Clone, Debug, Default, Parser)]
#[command(group = clap::ArgGroup::new("release-incompatible").multiple(true).conflicts_with("release"))]
pub(crate) struct ServeArgs {
    /// The arguments for the address the server will run on
    #[clap(flatten)]
    pub(crate) address: AddressArguments,

    /// Open the app in the default browser [default: true - unless cli settings are set]
    #[arg(long, default_missing_value="true", num_args=0..=1)]
    pub(crate) open: Option<bool>,

    /// Enable full hot reloading for the app [default: true - unless cli settings are set]
    #[clap(long, group = "release-incompatible")]
    pub(crate) hot_reload: Option<bool>,

    /// Configure always-on-top for desktop apps [default: true - unless cli settings are set]
    #[clap(long, default_missing_value = "true")]
    pub(crate) always_on_top: Option<bool>,

    /// Set cross-origin-policy to same-origin [default: false]
    #[clap(name = "cross-origin-policy")]
    #[clap(long)]
    pub(crate) cross_origin_policy: bool,

    /// Sets the interval in seconds that the CLI will poll for file changes on WSL.
    #[clap(long, default_missing_value = "2")]
    pub(crate) wsl_file_poll_interval: Option<u16>,

    /// Run the server in interactive mode
    #[arg(long, default_missing_value="true", num_args=0..=1, short = 'i')]
    pub(crate) interactive: Option<bool>,

    /// Enable Rust hot-patching instead of full rebuilds [default: false]
    ///
    /// This is quite experimental and may lead to unexpected segfaults or crashes in development.
    #[arg(long, default_value_t = false, alias = "hotpatch")]
    pub(crate) hot_patch: bool,

    /// Watch the filesystem for changes and trigger a rebuild [default: true]
    #[clap(long, default_missing_value = "true")]
    pub(crate) watch: Option<bool>,

    /// This flag only applies to fullstack builds. By default fullstack builds will run the server and client builds in parallel. This flag will force the build to run the server build first, then the client build. [default: false]
    #[clap(long)]
    pub(crate) force_sequential: bool,

    /// Exit the CLI after running into an error. This is mainly used to test hot patching internally
    #[clap(long)]
    #[clap(hide = true)]
    pub(crate) exit_on_error: bool,

    /// Platform-specific arguments for the build
    #[clap(flatten)]
    pub(crate) platform_args: CommandWithPlatformOverrides<PlatformServeArgs>,
}

impl Anonymized for ServeArgs {
    fn anonymized(&self) -> Value {
        json! {{
            "address": self.address.anonymized(),
            "open": self.open,
            "hot_reload": self.hot_reload,
            "always_on_top": self.always_on_top,
            "cross_origin_policy": self.cross_origin_policy,
            "wsl_file_poll_interval": self.wsl_file_poll_interval,
            "interactive": self.interactive,
            "hot_patch": self.hot_patch,
            "watch": self.watch,
            "force_sequential": self.force_sequential,
            "exit_on_error": self.exit_on_error,
            "platform_args": self.platform_args.anonymized(),
        }}
    }
}

impl ServeArgs {
    pub(crate) fn command_anonymized(&self) -> (String, Value) {
        let args = self.anonymized();

        ("serve".to_string(), args)
    }
}

#[derive(Clone, Debug, Default, Parser)]
pub(crate) struct PlatformServeArgs {
    #[clap(flatten)]
    pub(crate) targets: BuildArgs,

    /// Additional arguments to pass to the executable
    #[clap(long, default_value = "")]
    pub(crate) args: String,
}

impl Anonymized for PlatformServeArgs {
    fn anonymized(&self) -> Value {
        json! {{
            "targets": self.targets.anonymized(),
            "args": !self.args.is_empty(),
        }}
    }
}

impl ServeArgs {
    /// Start the tui, builder, etc by resolving the arguments and then running the actual top-level serve function
    ///
    /// Make sure not to do any intermediate logging since our tracing infra has now enabled much
    /// higher log levels
    ///
    /// We also set up proper panic handling since the TUI has a tendency to corrupt the terminal.
    pub(crate) async fn serve(self) -> Result<StructuredOutput> {
        if std::env::var("RUST_BACKTRACE").is_err() {
            std::env::set_var("RUST_BACKTRACE", "1");
        }

        let interactive = self.is_interactive_tty();

        // Redirect all logging the cli logger - if there's any pending after a panic, we flush it
        let mut tracer = TraceController::redirect(interactive);

        let res = crate::serve::serve_all(self, &mut tracer).await;

        // Kill the screen so we don't ruin the terminal
        _ = crate::serve::Output::remote_shutdown(interactive);

        // And drain the tracer as regular messages. All messages will be logged (including traces)
        // and then we can print the panic message
        if !matches!(res, Ok(_)) {
            tracer.shutdown_panic();
        }

        match res {
            Ok(_res) => Ok(StructuredOutput::Success),
            Err(e) => Err(e),
        }
    }

    /// Check if the server is running in interactive mode. This involves checking the terminal as well
    pub(crate) fn is_interactive_tty(&self) -> bool {
        use std::io::IsTerminal;
        std::io::stdout().is_terminal() && self.interactive.unwrap_or(true)
    }
}
