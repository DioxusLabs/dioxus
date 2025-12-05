use crate::BundleFormat;
use crate::Platform;
use crate::{cli::*, Renderer};
// use crate::RendererArg;
// use crate::PlatformAlias;
use target_lexicon::Triple;

const HELP_HEADING: &str = "Target Options";

/// A single target to build for
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub(crate) struct TargetArgs {
    /// Build platform: supports Web, MacOS, Windows, Linux, iOS, Android, and Server
    ///
    /// The platform implies a combination of the target alias, renderer, and bundle format flags.
    ///
    /// You should generally prefer to use the `--web`, `--webview`, or `--native` flags to set the renderer
    /// or the `--wasm`, `--macos`, `--windows`, `--linux`, `--ios`, or `--android` flags to set the target alias
    /// instead of this flag. The renderer, target alias, and bundle format will be inferred if you only pass one.
    #[clap(flatten)]
    pub(crate) platform: Platform,

    /// Which renderer to use? By default, this is usually inferred from the platform.
    #[clap(long, value_enum, help_heading = HELP_HEADING)]
    pub(crate) renderer: Option<Renderer>,

    /// The bundle format to target for the build: supports web, macos, windows, linux, ios, android, and server
    #[clap(long, value_enum, help_heading = HELP_HEADING)]
    pub(crate) bundle: Option<BundleFormat>,

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

    /// The name of the device we are hoping to upload to. By default, dx tries to upload to the active
    /// simulator. If the device name is passed, we will upload to that device instead.
    ///
    /// This performs a search among devices, and fuzzy matches might be found.
    #[arg(long, default_missing_value=Some("".into()), num_args=0..=1)]
    pub(crate) device: Option<String>,

    /// The base path the build will fetch assets relative to. This will override the
    /// base path set in the `dioxus` config.
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) base_path: Option<String>,

    /// Should dx attempt to codesign the app bundle?
    #[clap(long, default_value_t = false, help_heading = HELP_HEADING, num_args = 0..=1)]
    pub(crate) codesign: bool,

    /// The path to the Apple entitlements file to used to sign the resulting app bundle.
    ///
    /// On iOS, this is required for deploy to a device and some configurations in the simulator.
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) apple_entitlements: Option<PathBuf>,

    /// The Apple team ID to use when signing the app bundle.
    ///
    /// Usually this is an email or name associated with your Apple Developer account, usually in the
    /// format `Signing Name (GXTEAMID123)`.
    ///
    /// This is passed directly to the `codesign` tool.
    ///
    /// ```
    /// codesign --force --entitlements <entitlements_file> --sign <apple_team_id> <app_bundle>
    /// ```
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) apple_team_id: Option<String>,

    /// The folder where DX stores its temporary artifacts for things like hotpatching, build caches,
    /// window position, etc. This is meant to be stable within an invocation of the CLI, but you can
    /// persist it by setting this flag.
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) session_cache_dir: Option<PathBuf>,

    /// The target for the client build, used for specifying which target the server should end up in
    /// when merging `@client and @server` targets together.
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) client_target: Option<String>,

    /// Automatically pass `--features=js_cfg` when building for wasm targets. This is enabled by default.
    #[clap(long, default_value_t = true, help_heading = HELP_HEADING, num_args = 0..=1)]
    pub(crate) wasm_js_cfg: bool,

    /// The Windows subsystem to use when building for Windows targets. This can be either `CONSOLE` or `WINDOWS`.
    ///
    /// By default, DX uses `WINDOWS` since it assumes a GUI application, but you can override this behavior with this flag.
    ///
    /// See <https://learn.microsoft.com/en-us/cpp/build/reference/subsystem-specify-subsystem?view=msvc-170> for more information.
    #[clap(long, help_heading = HELP_HEADING)]
    pub(crate) windows_subsystem: Option<String>,

    /// Output raw JSON diagnostics from cargo instead of processing them [default: false]
    ///
    /// When enabled, cargo's JSON output will be relayed directly to stdout without any processing or formatting by DX.
    /// This is useful for integration with other tools that expect cargo's raw JSON format.
    #[clap(long, help_heading = HELP_HEADING)]
    #[serde(default)]
    pub(crate) raw_json_diagnostics: bool,

    #[clap(long, help_heading = HELP_HEADING)]
    #[serde(default)]
    pub(crate) disable_js_glue_shim: bool,
}

impl Anonymized for TargetArgs {
    fn anonymized(&self) -> Value {
        json! {{
            "renderer": self.renderer,
            "bundle": self.bundle,
            "platform": self.platform,
            "release": self.release,
            "package": self.package,
            "bin": self.bin,
            "example": self.example.is_some(),
            "profile": self.profile.is_some(),
            "features": !self.features.is_empty(),
            "no_default_features": self.no_default_features,
            "all_features": self.all_features,
            "target": self.target.as_ref().map(|t| t.to_string()),
            "skip_assets": self.skip_assets,
            "inject_loading_scripts": self.inject_loading_scripts,
            "wasm_split": self.wasm_split,
            "debug_symbols": self.debug_symbols,
            "device": self.device,
            "base_path": self.base_path.is_some(),
            "cargo_args": self.cargo_args.is_some(),
            "rustc_args": self.rustc_args.is_some(),
            "raw_json_diagnostics": self.raw_json_diagnostics,
        }}
    }
}
