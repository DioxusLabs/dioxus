use super::*;
use target_lexicon::Triple;

/// Information about the target to build
///
/// When running `dx serve / build / bundle` you can pass multiple targets to build for.
/// The args here are stand-in for `cargo rustc --args -- <more-args>`.
/// This lets you set up multiple projects to be ran in parallel.
///
/// The `@` sign is basically a task name and the args are passed to the task. We look for the task
/// in the Dioxus.toml and if it's not found, we just use the default args.
///
/// Any args preceeding the targets will be passed to *all* the targets, letting us keep some backwards
/// compatibility with the previous version of `dx serve`.
///
/// ```
/// dx serve --release
///     \ @client <target-args>
///     \ @server <target-args>
/// ```
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub(crate) struct TargetArgs {
    /// Build for nightly [default: false]
    #[clap(long)]
    pub(crate) nightly: bool,

    /// Build a example [default: ""]
    #[clap(long)]
    pub(crate) example: Option<String>,

    /// Build a binary [default: ""]
    #[clap(long)]
    pub(crate) bin: Option<String>,

    /// The package to build
    #[clap(short, long)]
    pub(crate) package: Option<String>,

    /// Space separated list of features to activate
    #[clap(long)]
    pub(crate) features: Vec<String>,

    /// The feature to use for the client in a fullstack app [default: "web"]
    #[clap(long)]
    pub(crate) client_features: Vec<String>,

    /// The feature to use for the server in a fullstack app [default: "server"]
    #[clap(long)]
    pub(crate) server_features: Vec<String>,

    /// Don't include the default features in the build
    #[clap(long)]
    pub(crate) no_default_features: bool,

    /// Are we building for a device or just the simulator.
    /// If device is false, then we'll build for the simulator
    #[clap(long)]
    pub(crate) device: Option<bool>,

    /// Rustc platform triple
    #[clap(long)]
    pub(crate) target: Option<Triple>,

    /// The target to build for the server.
    ///
    /// This can be different than the host allowing cross-compilation of the server. This is useful for
    /// platforms like Cloudflare Workers where the server is compiled to wasm and then uploaded to the edge.
    #[clap(long)]
    pub(crate) server_target: Option<Triple>,
}
