use std::{path::Path, str::FromStr};

use target_lexicon::Triple;

use super::{chained_command::ChainedCommand, *};
use crate::{Builder, DioxusCrate, Platform, PROFILE_SERVER};

/// Build the Rust Dioxus app and all of its assets.
///
/// Produces a final output build. For fullstack builds you need to build the server and client separately.
///
/// ```
/// dx build --platform web
/// dx build --platform server
/// ```
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub(crate) struct BuildArgs {
    /// This flag only applies to fullstack builds. By default fullstack builds will run the server and client builds in parallel. This flag will force the build to run the server build first, then the client build. [default: false]
    #[clap(long)]
    #[serde(default)]
    pub(crate) force_sequential: bool,

    /// Build the fullstack variant of this app, using that as the fileserver and backend
    ///
    /// This defaults to `false` but will be overridden to true if the `fullstack` feature is enabled.
    #[clap(long)]
    pub(crate) fullstack: bool,

    /// Run the ssg config of the app and generate the files
    #[clap(long)]
    pub(crate) ssg: bool,

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

    /// Information about the target to build
    ///
    /// These are the same args as `targets``
    #[clap(flatten)]
    pub(crate) args: TargetArgs,
}

impl BuildArgs {
    pub async fn build(self) -> Result<StructuredOutput> {
        tracing::info!("Building project...");

        let krate = DioxusCrate::new(&self.args).context("Failed to load Dioxus workspace")?;

        let bundle = Builder::start(&krate, &self)?.finish().await?;

        tracing::info!(path = ?bundle.build.root_dir(), "Build completed successfully! 🚀");

        Ok(StructuredOutput::BuildFinished {
            path: bundle.build.root_dir(),
        })
    }
}
