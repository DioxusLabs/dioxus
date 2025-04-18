use std::{backtrace::Backtrace, panic::AssertUnwindSafe};

use super::{chained::ChainedCommand, *};
use crate::{AddressArguments, BuildArgs, TraceController, PROFILE_SERVER};
use futures_util::FutureExt;
use once_cell::sync::OnceCell;
use target_lexicon::Triple;

/// Serve the project
///
/// `dx serve` takes cargo args by default, except with a required `--platform` arg:
///
/// ```
/// dx serve --example blah --target blah --platform android
/// ```
///
/// A simple serve:
/// ```
/// dx serve --platform web
/// ```
///
/// A serve with customized arguments:
///
/// ```
/// ```
///
/// As of dioxus 0.7, `dx serve` allows independent customization of the client and server builds,
/// allowing workspaces and removing any "magic" done to support ergonomic fullstack serving with
/// an plain `dx serve`. These require specifying more arguments like features since they won't be autodetected.
///
/// ```
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

    /// Additional arguments to pass to the executable
    #[clap(long)]
    pub(crate) args: Vec<String>,

    /// Sets the interval in seconds that the CLI will poll for file changes on WSL.
    #[clap(long, default_missing_value = "2")]
    pub(crate) wsl_file_poll_interval: Option<u16>,

    /// Run the server in interactive mode
    #[arg(long, default_missing_value="true", num_args=0..=1, short = 'i')]
    pub(crate) interactive: Option<bool>,

    /// Enable Rust hot-patching instead of full rebuilds [default: false]
    ///
    /// This is quite experimental and may lead to unexpected segfaults or crashes in development.
    #[arg(long, default_value_t = true)]
    pub(crate) hot_patch: bool,

    /// Enable fullstack mode [default: false]
    ///
    /// This is automatically detected from `dx serve` if the "fullstack" feature is enabled by default.
    pub(crate) fullstack: Option<bool>,

    /// The feature to use for the client in a fullstack app [default: "web"]
    #[clap(long)]
    pub(crate) client_features: Vec<String>,

    /// The feature to use for the server in a fullstack app [default: "server"]
    #[clap(long)]
    pub(crate) server_features: Vec<String>,

    /// Build with custom profile for the fullstack server
    #[clap(long, default_value_t = PROFILE_SERVER.to_string())]
    pub(crate) server_profile: String,

    /// The target to build for the server.
    ///
    /// This can be different than the host allowing cross-compilation of the server. This is useful for
    /// platforms like Cloudflare Workers where the server is compiled to wasm and then uploaded to the edge.
    #[clap(long)]
    pub(crate) server_target: Option<Triple>,

    /// Arguments for the build itself
    #[clap(flatten)]
    pub(crate) build_arguments: BuildArgs,

    /// A list of additional targets to build.
    ///
    /// Server and Client are special targets that integrate with `dx serve`, while `crate` is a generic.
    ///
    /// ```
    /// dx serve \
    ///     client --target aarch64-apple-darwin \
    ///     server --target wasm32-unknown-unknown \
    ///     crate --target aarch64-unknown-linux-gnu --package foo \
    ///     crate --target x86_64-unknown-linux-gnu --package bar
    /// ```
    #[command(subcommand)]
    pub(crate) targets: Option<TargetCmd>,
}

/// Launch a specific target
#[derive(Debug, Subcommand, Clone, Deserialize)]
#[command(subcommand_precedence_over_arg = true)]
pub(crate) enum TargetCmd {
    /// Specify the arguments for the client build
    #[clap(name = "client")]
    Client(ChainedCommand<BuildArgs, Self>),

    /// Specify the arguments for the server build
    #[clap(name = "server")]
    Server(ChainedCommand<BuildArgs, Self>),

    /// Specify the arguments for any number of additional targets
    #[clap(name = "crate")]
    Target(ChainedCommand<BuildArgs, Self>),
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

        struct SavedLocation {
            file: String,
            line: u32,
            column: u32,
        }
        static BACKTRACE: OnceCell<(Backtrace, Option<SavedLocation>)> = OnceCell::new();

        // We *don't* want printing here, since it'll break the tui and log ordering.
        //
        // We *will* re-emit the panic after we've drained the tracer, so our panic hook will simply capture the panic
        // and save it.
        std::panic::set_hook(Box::new(move |panic_info| {
            _ = BACKTRACE.set((
                Backtrace::capture(),
                panic_info.location().map(|l| SavedLocation {
                    file: l.file().to_string(),
                    line: l.line(),
                    column: l.column(),
                }),
            ));
        }));

        let interactive = self.is_interactive_tty();

        // Redirect all logging the cli logger - if there's any pending after a panic, we flush it
        let mut tracer = TraceController::redirect(interactive);

        let res = AssertUnwindSafe(crate::serve::serve_all(self, &mut tracer))
            .catch_unwind()
            .await;

        // Kill the screen so we don't ruin the terminal
        _ = crate::serve::Output::remote_shutdown(interactive);

        // And drain the tracer as regular messages. All messages will be logged (including traces)
        // and then we can print the panic message
        if !matches!(res, Ok(Ok(_))) {
            tracer.shutdown_panic();
        }

        match res {
            Ok(Ok(_res)) => Ok(StructuredOutput::Success),
            Ok(Err(e)) => Err(e),
            Err(panic_err) => {
                // And then print the panic itself.
                let as_str = if let Some(p) = panic_err.downcast_ref::<String>() {
                    p.as_ref()
                } else if let Some(p) = panic_err.downcast_ref::<&str>() {
                    p.as_ref()
                } else {
                    "<unknown panic>"
                };

                // Attempt to emulate the default panic hook
                let message = BACKTRACE
                    .get()
                    .map(|(back, location)| {
                        let location_display = location
                            .as_ref()
                            .map(|l| format!("{}:{}:{}", l.file, l.line, l.column))
                            .unwrap_or_else(|| "<unknown>".to_string());

                        let mut backtrace_display = back.to_string();

                        // split at the line that ends with ___rust_try for short backtraces
                        if std::env::var("RUST_BACKTRACE") == Ok("1".to_string()) {
                            backtrace_display = backtrace_display
                                .split(" ___rust_try\n")
                                .next()
                                .map(|f| format!("{f} ___rust_try"))
                                .unwrap_or_default();
                        }

                        format!("dx serve panicked at {location_display}\n{as_str}\n{backtrace_display} ___rust_try")
                    })
                    .unwrap_or_else(|| format!("dx serve panicked: {as_str}"));

                Err(crate::error::Error::CapturedPanic(message))
            }
        }
    }

    /// Check if the server is running in interactive mode. This involves checking the terminal as well
    pub(crate) fn is_interactive_tty(&self) -> bool {
        use std::io::IsTerminal;
        std::io::stdout().is_terminal() && self.interactive.unwrap_or(true)
    }
}
