use crate::{cli::*, AppBuilder, BuildRequest, Workspace};
use crate::{BuildMode, Platform};
use target_lexicon::Triple;

/// Build the Rust Dioxus app and all of its assets.
///
/// Produces a final output build. For fullstack builds you need to build the server and client separately.
/// ```
/// dx build --platform web
/// dx build --platform server
/// ```
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub(crate) struct BuildArgs {
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

    /// Rustc platform triple
    #[clap(long)]
    pub(crate) target: Option<Triple>,

    /// Extra arguments passed to `cargo`
    ///
    /// To see a list of args, run `cargo rustc --help`
    ///
    /// This can include stuff like, "--locked", "--frozen", etc. Note that `dx` sets many of these
    /// args directly from other args in this command.
    #[clap(value_delimiter = ',')]
    pub(crate) cargo_args: Vec<String>,

    /// Extra arguments passed to `rustc`. This can be used to customize the linker, or other flags.
    ///
    /// For example, specifign `dx build --rustc-args "-Clink-arg=-Wl,-blah"` will pass "-Clink-arg=-Wl,-blah"
    /// to the underlying the `cargo rustc` command:
    ///
    /// cargo rustc -- -Clink-arg=-Wl,-blah
    ///
    #[clap(long)]
    pub(crate) rustc_args: Option<String>,

    /// Skip collecting assets from dependencies [default: false]
    #[clap(long)]
    #[serde(default)]
    pub(crate) skip_assets: bool,

    /// Inject scripts to load the wasm and js files for your dioxus app if they are not already present [default: true]
    #[clap(long, default_value_t = true)]
    pub(crate) inject_loading_scripts: bool,

    /// Experimental: Bundle split the wasm binary into multiple chunks based on `#[wasm_split]` annotations [default: false]
    #[clap(long, default_value_t = false)]
    pub(crate) wasm_split: bool,

    /// Generate debug symbols for the wasm binary [default: true]
    ///
    /// This will make the binary larger and take longer to compile, but will allow you to debug the
    /// wasm binary
    #[clap(long, default_value_t = true)]
    pub(crate) debug_symbols: bool,

    /// Use the cranelift backend to compile the app [default: false]
    ///
    /// This can speed up compile times by up to 100% but is experimental within the compiler.
    #[clap(long)]
    pub(crate) cranelift: bool,

    /// Are we building for a device or just the simulator.
    /// If device is false, then we'll build for the simulator
    #[clap(long)]
    pub(crate) device: Option<bool>,
}

impl BuildArgs {
    pub async fn build(self) -> Result<StructuredOutput> {
        tracing::info!("Building project...");

        let workspace = Workspace::current().await?;
        let build = BuildRequest::new(&self, workspace)
            .await
            .context("Failed to load Dioxus workspace")?;

        AppBuilder::start(&build, BuildMode::Base)?
            .finish_build()
            .await?;

        tracing::info!(path = ?build.root_dir(), "Build completed successfully! 🚀");

        Ok(StructuredOutput::BuildFinished {
            path: build.root_dir(),
        })
    }
}
