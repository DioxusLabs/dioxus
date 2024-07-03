use dioxus_cli_config::Platform;
use dioxus_cli_config::ServeArguments;

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

    /// This flag only applies to fullstack builds. By default fullstack builds will run the server and client builds in parallel. This flag will force the build to run the server build first, then the client build. [default: false]
    #[clap(long)]
    #[serde(default)]
    pub force_sequential: bool,

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

    /// Inject scripts to load the wasm and js files for your dioxus app if they are not already present [default: true]
    #[clap(long, default_value_t = true)]
    pub inject_loading_scripts: bool,
}

impl From<ConfigOptsServe> for ConfigOptsBuild {
    fn from(serve: ConfigOptsServe) -> Self {
        serve.build_arguments
    }
}

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[command(group = clap::ArgGroup::new("release-incompatible").multiple(true).conflicts_with("release"))]
pub struct ConfigOptsServe {
    /// Arguments for the serve command
    #[clap(flatten)]
    pub(crate) server_arguments: ServeArguments,

    /// Arguments for the dioxus build
    #[clap(flatten)]
    pub(crate) build_arguments: ConfigOptsBuild,

    // TODO: Somehow make this default to `true` if the flag was provided. e.g. `dx serve --open`
    // Currently it requires a value: `dx serve --open true`
    /// Open the app in the default browser [default: false - unless project or global settings are set]
    #[clap(long)]
    pub open: Option<bool>,

    // TODO: See `open` field
    /// Enable full hot reloading for the app [default: true - unless project or global settings are set]
    #[clap(long, group = "release-incompatible")]
    pub hot_reload: Option<bool>,

    /// Set cross-origin-policy to same-origin [default: false]
    #[clap(name = "cross-origin-policy")]
    #[clap(long)]
    #[serde(default)]
    pub cross_origin_policy: bool,

    /// Additional arguments to pass to the executable
    #[clap(long)]
    pub args: Vec<String>,
}

/// Config options for the bundling system.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub struct ConfigOptsBundle {
    /// The arguments for the dioxus build
    #[clap(flatten)]
    pub build_arguments: ConfigOptsBuild,
}
