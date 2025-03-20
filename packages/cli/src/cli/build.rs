use std::{path::Path, str::FromStr};

use target_lexicon::Triple;

use super::*;
use crate::{Builder, DioxusCrate, Platform, PROFILE_SERVER};

/// Build the Rust Dioxus app and all of its assets.
///
/// Produces a final output bundle designed to be run on the target platform.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub(crate) struct BuildArgs {
    /// Build in release mode [default: false]
    #[clap(long, short)]
    #[serde(default)]
    pub(crate) release: bool,

    /// This flag only applies to fullstack builds. By default fullstack builds will run the server and client builds in parallel. This flag will force the build to run the server build first, then the client build. [default: false]
    #[clap(long)]
    #[serde(default)]
    pub(crate) force_sequential: bool,

    /// Build the app with custom a profile
    #[clap(long)]
    pub(crate) profile: Option<String>,

    /// Build with custom profile for the fullstack server
    #[clap(long, default_value_t = PROFILE_SERVER.to_string())]
    pub(crate) server_profile: String,

    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long, value_enum)]
    pub(crate) platform: Option<Platform>,

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

    /// Extra arguments passed to cargo build
    #[clap(last = true)]
    pub(crate) cargo_args: Vec<String>,

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

    /// Information about the target to build
    #[clap(flatten)]
    pub(crate) target_args: TargetArgs,
}

impl BuildArgs {
    pub async fn run_cmd(self) -> Result<StructuredOutput> {
        tracing::info!("Building project...");

        let krate =
            DioxusCrate::new(&self.target_args).context("Failed to load Dioxus workspace")?;

        let bundle = Builder::start(&krate, self)?.finish().await?;

        tracing::info!(path = ?bundle.build.root_dir(), "Build completed successfully! 🚀");

        Ok(StructuredOutput::BuildFinished {
            path: bundle.build.root_dir(),
        })
    }
}
