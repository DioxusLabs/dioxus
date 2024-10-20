use super::*;

/// Information about the target to build
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

    /// The architecture to build for [default: "native"]
    ///
    /// Can either be `arm | arm64 | x86 | x86_64 | native`
    #[clap(long)]
    pub(crate) arch: Option<String>,

    /// Are we building for a device or just the simulator
    /// If device is false, then we'll build for the simulator
    #[clap(long)]
    pub(crate) device: Option<bool>,

    /// Rustc platform triple
    #[clap(long)]
    pub(crate) target: Option<String>,
}
