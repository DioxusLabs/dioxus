use crate::cli::*;
use crate::BundleFormat;
use crate::Platform;
use crate::RendererArg;
use crate::TargetAlias;
use target_lexicon::Triple;

const HELP_HEADING: &str = "Target Options";

/// A single target to build for
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub(crate) struct TargetArgs {
    /// The target alias to use for this build. Supports wasm, macos, windows, linux, ios, android, and host [default: "host"]
    #[clap(flatten)]
    pub(crate) target_alias: TargetAlias,

    /// Build renderer: supports web, webview, native, server, and liveview
    #[clap(flatten)]
    pub(crate) renderer: RendererArg,

    /// The bundle format to target for the build: supports web, macos, windows, linux, ios, android, and server
    #[clap(long, value_enum, help_heading = HELP_HEADING)]
    pub(crate) bundle: Option<BundleFormat>,

    /// Build platform: supports Web, MacOS, Windows, Linux, iOS, Android, and Server
    ///
    /// The platform implies a combination of the target alias, renderer, and bundle format flags.
    ///
    /// You should generally prefer to use the `--web`, `--webview`, or `--native` flags to set the renderer
    /// or the `--wasm`, `--macos`, `--windows`, `--linux`, `--ios`, or `--android` flags to set the target alias
    /// instead of this flag. The renderer, target alias, and bundle format will be inferred if you only pass one.
    #[clap(long, value_enum, help_heading = HELP_HEADING)]
    pub(crate) platform: Option<Platform>,

    /// Build in release mode [default: false]
    #[clap(long, short, help_heading = HELP_HEADING)]
    #[serde(default)]
    pub(crate) release: bool,

    /// The package to build
    #[clap(short, long, help_heading = HELP_HEADING)]
    pub(crate) package: Option<String>,

    /// Build a specific binary [default: ""]
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) bin: Option<String>,

    /// Build a specific example [default: ""]
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) example: Option<String>,

    /// Build the app with custom a profile
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) profile: Option<String>,

    /// Space separated list of features to activate
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) features: Vec<String>,

    /// Don't include the default features in the build
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) no_default_features: bool,

    /// Include all features in the build
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) all_features: bool,

    /// Rustc platform triple
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) target: Option<Triple>,

    /// Extra arguments passed to `cargo`
    ///
    /// To see a list of args, run `cargo rustc --help`
    ///
    /// This can include stuff like, "--locked", "--frozen", etc. Note that `dx` sets many of these
    /// args directly from other args in this command.
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) cargo_args: Option<String>,

    /// Extra arguments passed to `rustc`. This can be used to customize the linker, or other flags.
    ///
    /// For example, specifign `dx build --rustc-args "-Clink-arg=-Wl,-blah"` will pass "-Clink-arg=-Wl,-blah"
    /// to the underlying the `cargo rustc` command:
    ///
    /// cargo rustc -- -Clink-arg=-Wl,-blah
    ///
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) rustc_args: Option<String>,

    /// Skip collecting assets from dependencies [default: false]
    #[clap(long, help_heading = HELP_HEADING)]
    #[serde(default)]
    pub(crate) skip_assets: bool,

    /// Inject scripts to load the wasm and js files for your dioxus app if they are not already present [default: true]
    #[clap(long, default_value_t = true, help_heading = HELP_HEADING, num_args = 0..=1)]
    pub(crate) inject_loading_scripts: bool,

    /// Experimental: Bundle split the wasm binary into multiple chunks based on `#[wasm_split]` annotations [default: false]
    #[clap(long, default_value_t = false, help_heading = HELP_HEADING)]
    pub(crate) wasm_split: bool,

    /// Generate debug symbols for the wasm binary [default: true]
    ///
    /// This will make the binary larger and take longer to compile, but will allow you to debug the
    /// wasm binary
    #[clap(long, default_value_t = true, help_heading = HELP_HEADING, num_args = 0..=1)]
    pub(crate) debug_symbols: bool,

    /// Are we building for a device or just the simulator.
    /// If device is false, then we'll build for the simulator
    #[clap(long, default_value_t = false, help_heading = HELP_HEADING)]
    pub(crate) device: bool,

    /// The base path the build will fetch assets relative to. This will override the
    /// base path set in the `dioxus` config.
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) base_path: Option<String>,
}
