use super::*;
use crate::Platform;
use target_lexicon::Triple;

/// Information about the target to build
///
/// This should be enough information to build the target.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[group(required = false, multiple = true)]
pub(crate) struct TargetArgs {
    #[clap(long)]
    pub(crate) name: Option<String>,

    /// Build for nightly [default: false]
    #[clap(long)]
    pub(crate) nightly: bool,

    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long, value_enum)]
    pub(crate) platform: Option<Platform>,

    /// Build in release mode [default: false]
    #[clap(long, short)]
    #[serde(default)]
    pub(crate) release: bool,

    /// The package to build
    #[clap(short, long)]
    pub(crate) package: Option<String>,

    /// Build a specific binary [default: ""]
    #[clap(long)]
    pub(crate) bin: Option<String>,

    /// Build a specific example [default: ""]
    #[clap(long)]
    pub(crate) example: Option<String>,

    /// Build the app with custom a profile
    #[clap(long)]
    pub(crate) profile: Option<String>,

    /// Space separated list of features to activate
    #[clap(long)]
    pub(crate) features: Vec<String>,

    /// Don't include the default features in the build
    #[clap(long)]
    pub(crate) no_default_features: bool,

    /// Include all features in the build
    #[clap(long)]
    pub(crate) all_features: bool,

    /// Are we building for a device or just the simulator.
    /// If device is false, then we'll build for the simulator
    #[clap(long)]
    pub(crate) device: Option<bool>,

    /// Rustc platform triple
    #[clap(long)]
    pub(crate) target: Option<Triple>,
    // todo -- make a subcommand called "--" that takes all the remaining args
    /// Extra arguments passed to `rustc`
    ///
    /// cargo rustc -- -Clinker
    #[clap(value_delimiter = ',')]
    pub(crate) cargo_args: Vec<String>,
    // #[clap(last = true)]
    // pub(crate) cargo_args: Vec<String>,
}

struct CargoArgs {
    args: Vec<String>,
}
