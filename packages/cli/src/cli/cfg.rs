use dioxus_cli_config::Platform;

use super::*;

/// Config options for the build system.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub struct ConfigOptsBuild {
    /// Build in release mode [default: false]
    #[clap(long, short)]
    #[serde(default)]
    pub release: bool,

    /// This flag only applies to fullstack builds. By default fullstack builds will run with something in between debug and release mode. This flag will force the build to run in debug mode. [default: false]
    #[clap(long)]
    #[serde(default)]
    pub force_debug: bool,

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

    /// Skip collecting assets from dependencies [default: false]
    #[clap(long)]
    #[serde(default)]
    pub skip_assets: bool,

    /// Space separated list of features to activate
    #[clap(long)]
    pub features: Option<Vec<String>>,

    /// The feature to use for the client in a fullstack app [default: "web"]
    #[clap(long, default_value_t = { "web".to_string() })]
    pub client_feature: String,

    /// The feature to use for the server in a fullstack app [default: "server"]
    #[clap(long, default_value_t = { "server".to_string() })]
    pub server_feature: String,

    /// Rustc platform triple
    #[clap(long)]
    pub target: Option<String>,

    /// Extra arguments passed to cargo build
    #[clap(last = true)]
    pub cargo_args: Vec<String>,
}

impl From<ConfigOptsServe> for ConfigOptsBuild {
    fn from(serve: ConfigOptsServe) -> Self {
        Self {
            target: serve.target,
            release: serve.release,
            verbose: serve.verbose,
            example: serve.example,
            profile: serve.profile,
            platform: serve.platform,
            features: serve.features,
            client_feature: serve.client_feature,
            server_feature: serve.server_feature,
            skip_assets: serve.skip_assets,
            force_debug: serve.force_debug,
            cargo_args: serve.cargo_args,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[command(group = clap::ArgGroup::new("release-incompatible").multiple(true).conflicts_with("release"))]
pub struct ConfigOptsServe {
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

    /// This flag only applies to fullstack builds. By default fullstack builds will run with something in between debug and release mode. This flag will force the build to run in debug mode. [default: false]
    #[clap(long)]
    #[serde(default)]
    pub force_debug: bool,

    // Use verbose output [default: false]
    #[clap(long)]
    #[serde(default)]
    pub verbose: bool,

    /// Build with custom profile
    #[clap(long)]
    pub profile: Option<String>,

    /// Build platform: support Web, Desktop, and Fullstack [default: "default_platform"]
    #[clap(long, value_enum)]
    pub platform: Option<Platform>,

    /// Build with hot reloading rsx. Will not work with release builds. [default: false]
    #[clap(long)]
    #[clap(group = "release-incompatible")]
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

    /// Skip collecting assets from dependencies [default: false]
    #[clap(long)]
    #[serde(default)]
    pub skip_assets: bool,

    /// The feature to use for the client in a fullstack app [default: "web"]
    #[clap(long, default_value_t = { "web".to_string() })]
    pub client_feature: String,

    /// The feature to use for the server in a fullstack app [default: "server"]
    #[clap(long, default_value_t = { "server".to_string() })]
    pub server_feature: String,

    /// Rustc platform triple
    #[clap(long)]
    pub target: Option<String>,

    /// Extra arguments passed to cargo build
    #[clap(last = true)]
    pub cargo_args: Vec<String>,
}

/// Config options for the bundling system.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub struct ConfigOptsBundle {
    /// Build in release mode [default: false]
    #[clap(long, short)]
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

    /// Rustc platform triple
    #[clap(long)]
    pub target: Option<String>,

    /// Extra arguments passed to cargo build
    #[clap(last = true)]
    pub cargo_args: Vec<String>,
}
