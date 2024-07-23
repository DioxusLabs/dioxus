use anyhow::Context;
use dioxus_cli_config::Platform;

use crate::{builder::BuildRequest, dioxus_crate::DioxusCrate};

use super::*;

/// Information about the target to build
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub struct TargetArgs {
    /// Build for nightly [default: false]
    #[clap(long)]
    pub nightly: bool,

    /// Build a example [default: ""]
    #[clap(long)]
    pub example: Option<String>,

    /// Build a binary [default: ""]
    #[clap(long)]
    pub bin: Option<String>,

    /// The package to build
    #[clap(long)]
    pub package: Option<String>,

    /// Space separated list of features to activate
    #[clap(long)]
    pub features: Vec<String>,

    /// The feature to use for the client in a fullstack app [default: "web"]
    #[clap(long, default_value_t = { "web".to_string() })]
    pub client_feature: String,

    /// The feature to use for the server in a fullstack app [default: "server"]
    #[clap(long, default_value_t = { "server".to_string() })]
    pub server_feature: String,

    /// Rustc platform triple
    #[clap(long)]
    pub target: Option<String>,
}

/// Build the Rust Dioxus app and all of its assets.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "build")]
pub struct Build {
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

    /// Extra arguments passed to cargo build
    #[clap(last = true)]
    pub cargo_args: Vec<String>,

    /// Inject scripts to load the wasm and js files for your dioxus app if they are not already present [default: true]
    #[clap(long, default_value_t = true)]
    pub inject_loading_scripts: bool,

    /// Information about the target to build
    #[clap(flatten)]
    pub target_args: TargetArgs,
}

impl Build {
    pub fn resolve(&mut self, dioxus_crate: &mut DioxusCrate) -> Result<()> {
        // Inherit the platform from the defaults
        let platform = self
            .platform
            .unwrap_or_else(|| self.auto_detect_platform(dioxus_crate));
        self.platform = Some(platform);

        // Add any features required to turn on the platform we are building for
        self.target_args
            .features
            .extend(dioxus_crate.features_for_platform(platform));

        Ok(())
    }

    pub async fn build(&mut self, dioxus_crate: &mut DioxusCrate) -> Result<()> {
        self.resolve(dioxus_crate)?;
        let build_requests = BuildRequest::create(false, dioxus_crate, self.clone());
        BuildRequest::build_all_parallel(build_requests).await?;
        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut dioxus_crate =
            DioxusCrate::new(&self.target_args).context("Failed to load Dioxus workspace")?;
        self.build(&mut dioxus_crate).await?;
        Ok(())
    }

    fn auto_detect_platform(&self, resolved: &DioxusCrate) -> Platform {
        for dioxus in resolved.krates.krates_by_name("dioxus") {
            let Some(features) = resolved.krates.get_enabled_features(dioxus.kid) else {
                continue;
            };

            if let Some(platform) = features
                .iter()
                .find_map(|platform| platform.parse::<Platform>().ok())
            {
                return platform;
            }
        }

        resolved.dioxus_config.application.default_platform
    }

    /// Get the platform from the build arguments
    pub fn platform(&self) -> Platform {
        self.platform.unwrap_or_default()
    }
}
