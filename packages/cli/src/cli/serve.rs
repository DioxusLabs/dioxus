use super::{chained::ChainedCommand, *};
use crate::{AddressArguments, BuildArgs, PROFILE_SERVER};
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
    pub(crate) async fn serve(self) -> Result<StructuredOutput> {
        crate::serve::serve_all(self).await?;
        Ok(StructuredOutput::Success)
    }

    /// Check if the server is running in interactive mode. This involves checking the terminal as well
    pub(crate) fn is_interactive_tty(&self) -> bool {
        use std::io::IsTerminal;
        std::io::stdout().is_terminal() && self.interactive.unwrap_or(true)
    }
}
