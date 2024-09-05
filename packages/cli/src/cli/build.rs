use super::*;
use crate::builder::{Builder, Platform};
use crate::dioxus_crate::DioxusCrate;
use anyhow::Context;
use std::str::FromStr;

/// Build the Rust Dioxus app and all of its assets.
///
/// Produces a final output bundle designed to be run on the target platform.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "build")]
pub struct BuildArgs {
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

    /// Build the fullstack variant of this app, using that as the fileserver and backend
    ///
    /// This defaults to `false` but will be overriden to true if the `fullstack` feature is enabled.
    #[clap(long)]
    pub fullstack: bool,

    /// Run the ssg config of the app and generate the files
    #[clap(long)]
    pub ssg: bool,

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
    #[clap(short, long)]
    pub package: Option<String>,

    /// Space separated list of features to activate
    #[clap(long)]
    pub features: Vec<String>,

    /// The feature to use for the client in a fullstack app [default: "web"]
    #[clap(long)]
    pub client_feature: Option<String>,

    /// The feature to use for the server in a fullstack app [default: "server"]
    #[clap(long)]
    pub server_feature: Option<String>,

    /// The architecture to build for [default: "native"]
    ///
    /// Can either be `arm | arm64 | x86 | x86_64 | native`
    #[clap(long)]
    pub arch: Option<String>,

    /// Rustc platform triple
    #[clap(long)]
    pub target: Option<String>,
}

impl BuildArgs {
    pub async fn run(mut self) -> anyhow::Result<()> {
        let mut dioxus_crate =
            DioxusCrate::new(&self.target_args).context("Failed to load Dioxus workspace")?;

        self.build(&mut dioxus_crate).await?;

        Ok(())
    }

    pub async fn build(&mut self, dioxus_crate: &mut DioxusCrate) -> Result<()> {
        self.resolve(dioxus_crate)?;

        // todo: probably want to consume the logs from the builder here, instead of just waiting for it to finish
        Builder::start(dioxus_crate, self.clone())?
            .wait_for_finish()
            .await;

        Ok(())
    }

    /// Update the arguments of the CLI by inspecting the DioxusCrate itself and learning about how
    /// the user has configured their app.
    ///
    /// IE if they've specified "fullstack" as a feature on `dioxus`, then we want to build the
    /// fullstack variant even if they omitted the `--fullstack` flag.
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

    pub fn auto_detect_client_platform(
        &self,
        resolved: &DioxusCrate,
    ) -> (Option<String>, Platform) {
        self.find_dioxus_feature(resolved, |platform| {
            matches!(platform, Platform::Web | Platform::Desktop)
        })
        .unwrap_or_else(|| (Some("web".to_string()), Platform::Web))
    }

    pub fn auto_detect_server_feature(&self, resolved: &DioxusCrate) -> Option<String> {
        self.find_dioxus_feature(resolved, |platform| matches!(platform, Platform::Server))
            .map(|(feature, _)| feature)
            .unwrap_or_else(|| Some("server".to_string()))
    }

    fn auto_detect_platform(&self, resolved: &DioxusCrate) -> Platform {
        self.auto_detect_platform_with_filter(resolved, |_| true).1
    }

    fn auto_detect_platform_with_filter(
        &self,
        resolved: &DioxusCrate,
        filter_platform: fn(&Platform) -> bool,
    ) -> (Option<String>, Platform) {
        self.find_dioxus_feature(resolved, filter_platform)
            .unwrap_or_else(|| {
                let default_platform = resolved.dioxus_config.application.default_platform;

                (Some(default_platform.to_string()), default_platform)
            })
    }

    fn find_dioxus_feature<P: FromStr>(
        &self,
        resolved: &DioxusCrate,
        filter_platform: fn(&P) -> bool,
    ) -> Option<(Option<String>, P)> {
        // First check the enabled features for any renderer enabled
        for dioxus in resolved.krates.krates_by_name("dioxus") {
            let Some(features) = resolved.krates.get_enabled_features(dioxus.kid) else {
                continue;
            };

            if let Some(platform) = features
                .iter()
                .find_map(|platform| platform.parse::<P>().ok())
                .filter(filter_platform)
            {
                return Some((None, platform));
            }
        }

        // Then check the features that might get enabled
        if let Some(platform) = resolved
            .package()
            .features
            .iter()
            .find_map(|(feature, enables)| {
                enables
                    .iter()
                    .find_map(|f| {
                        f.strip_prefix("dioxus/")
                            .or_else(|| feature.strip_prefix("dep:dioxus/"))
                            .and_then(|f| f.parse::<P>().ok())
                            .filter(filter_platform)
                    })
                    .map(|platform| (Some(feature.clone()), platform))
            })
        {
            return Some(platform);
        }

        None
    }

    /// Get the platform from the build arguments
    pub fn platform(&self) -> Platform {
        self.platform.unwrap_or_default()
    }
}
