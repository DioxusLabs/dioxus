use clap::ValueEnum;
use serde::Serialize;

use super::*;

/// Config options for the build system.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub struct ConfigOptsBuild {
    /// The index HTML file to drive the bundling process [default: index.html]
    #[arg(long)]
    pub target: Option<PathBuf>,

    /// Build in release mode [default: false]
    #[clap(long)]
    #[serde(default)]
    pub release: bool,

    // Use verbose output [default: false]
    #[clap(long)]
    #[serde(default)]
    pub verbose: bool,

    /// Build a example [default: ""]
    #[clap(long)]
    pub example: Option<String>,

    /// Build with custom profile
    #[clap(long)]
    pub profile: Option<String>,

    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long, value_enum)]
    pub platform: Option<Platform>,

    /// Space separated list of features to activate
    #[clap(long)]
    pub features: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub struct ConfigOptsServe {
    /// The index HTML file to drive the bundling process [default: index.html]
    #[arg(short, long)]
    pub target: Option<PathBuf>,

    /// Port of dev server
    #[clap(long)]
    #[clap(default_value_t = 8080)]
    pub port: u16,

    /// Open the app in the default browser [default: false]
    #[clap(long)]
    #[serde(default)]
    pub open: bool,

    /// Build a example [default: ""]
    #[clap(long)]
    pub example: Option<String>,

    /// Build in release mode [default: false]
    #[clap(long)]
    #[serde(default)]
    pub release: bool,

    // Use verbose output [default: false]
    #[clap(long)]
    #[serde(default)]
    pub verbose: bool,

    /// Build with custom profile
    #[clap(long)]
    pub profile: Option<String>,

    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long, value_enum)]
    pub platform: Option<Platform>,

    /// Build with hot reloading rsx [default: false]
    #[clap(long)]
    #[serde(default)]
    pub hot_reload: bool,

    /// Set cross-origin-policy to same-origin [default: false]
    #[clap(name = "cross-origin-policy")]
    #[clap(long)]
    #[serde(default)]
    pub cross_origin_policy: bool,

    /// Space separated list of features to activate
    #[clap(long)]
    pub features: Option<Vec<String>>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Deserialize, Debug)]
pub enum Platform {
    #[clap(name = "web")]
    #[serde(rename = "web")]
    Web,
    #[clap(name = "desktop")]
    #[serde(rename = "desktop")]
    Desktop,
}

/// Config options for the bundling system.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub struct ConfigOptsBundle {
    /// Build in release mode [default: false]
    #[clap(long)]
    #[serde(default)]
    pub release: bool,

    // Use verbose output [default: false]
    #[clap(long)]
    #[serde(default)]
    pub verbose: bool,

    /// Build a example [default: ""]
    #[clap(long)]
    pub example: Option<String>,

    /// Build with custom profile
    #[clap(long)]
    pub profile: Option<String>,

    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long)]
    pub platform: Option<String>,

    /// Space separated list of features to activate
    #[clap(long)]
    pub features: Option<Vec<String>>,
}
