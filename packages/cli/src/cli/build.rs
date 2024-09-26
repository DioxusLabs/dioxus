use super::*;
use crate::{dioxus_crate::DioxusCrate, Builder, Platform};

/// Build the Rust Dioxus app and all of its assets.
///
/// Produces a final output bundle designed to be run on the target platform.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "build")]
pub(crate) struct BuildArgs {
    /// Build in release mode [default: false]
    #[clap(long, short)]
    #[serde(default)]
    pub(crate) release: bool,

    /// This flag only applies to fullstack builds. By default fullstack builds will run with something in between debug and release mode. This flag will force the build to run in debug mode. [default: false]
    #[clap(long)]
    #[serde(default)]
    pub(crate) force_debug: bool,

    /// This flag only applies to fullstack builds. By default fullstack builds will run the server and client builds in parallel. This flag will force the build to run the server build first, then the client build. [default: false]
    #[clap(long)]
    #[serde(default)]
    pub(crate) force_sequential: bool,

    /// Use verbose output [default: false]
    #[clap(long)]
    #[serde(default)]
    pub(crate) verbose: bool,

    /// Use trace output [default: false]
    #[clap(long)]
    #[serde(default)]
    pub(crate) trace: bool,

    /// Pass -Awarnings to the cargo build
    #[clap(long)]
    #[serde(default)]
    pub(crate) silent: bool,

    /// Build the app with custom a profile
    #[clap(long)]
    pub(crate) profile: Option<String>,

    /// Build with custom profile for the fullstack server
    #[clap(long)]
    pub(crate) server_profile: Option<String>,

    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long, value_enum)]
    pub(crate) platform: Option<Platform>,

    /// Build the fullstack variant of this app, using that as the fileserver and backend
    ///
    /// This defaults to `false` but will be overriden to true if the `fullstack` feature is enabled.
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

    /// Information about the target to build
    #[clap(flatten)]
    pub(crate) target_args: TargetArgs,
}

impl BuildArgs {
    pub(crate) async fn run(mut self) -> Result<()> {
        let krate =
            DioxusCrate::new(&self.target_args).context("Failed to load Dioxus workspace")?;

        self.build(&krate).await?;

        Ok(())
    }

    pub(crate) async fn build(&mut self, dioxus_crate: &DioxusCrate) -> Result<()> {
        self.resolve(dioxus_crate)?;

        // todo: probably want to consume the logs from the builder here, instead of just waiting for it to finish
        let bundle = Builder::start(dioxus_crate, self.clone())?.finish().await?;
        let destination = dioxus_crate.bundle_dir(self.platform());
        bundle.finish(destination)?;

        Ok(())
    }

    /// Update the arguments of the CLI by inspecting the DioxusCrate itself and learning about how
    /// the user has configured their app.
    ///
    /// IE if they've specified "fullstack" as a feature on `dioxus`, then we want to build the
    /// fullstack variant even if they omitted the `--fullstack` flag.
    pub(crate) fn resolve(&mut self, dioxus_crate: &DioxusCrate) -> Result<()> {
        // Inherit the platform from the args, or auto-detect it
        if self.platform.is_none() {
            let platform = self.auto_detect_platform(dioxus_crate).ok_or_else(|| {
                anyhow::anyhow!("No platform was specified and could not be auto-detected. Please specify a platform with `--platform <platform>`")
            })?;
            self.platform = Some(platform);
        }

        let platform = self.platform.unwrap();

        // Add any features required to turn on the client
        self.target_args
            .client_features
            .extend(dioxus_crate.feature_for_platform(platform));

        // Add any features required to turn on the server
        // This won't take effect in the server is not built, so it's fine to just set it here even if it's not used
        self.target_args
            .server_features
            .extend(dioxus_crate.feature_for_platform(Platform::Server));

        // Make sure we set the fullstack platform so we actually build the fullstack variant
        // Users need to enable "fullstack" in their default feature set.
        // todo(jon): fullstack *could* be a feature of the app, but right now we're assuming it's always enabled
        self.fullstack = self.fullstack || self.has_dioxus_feature(dioxus_crate, "fullstack");

        // Make sure we have a server feature if we're building a fullstack app
        //
        // todo(jon): eventually we want to let users pass a `--server <crate>` flag to specify a package to use as the server
        // however, it'll take some time to support that and we don't have a great RPC binding layer between the two yet
        if self.fullstack && self.target_args.server_features.is_empty() {
            return Err(anyhow::anyhow!("Fullstack builds require a server feature on the target crate. Add a `server` feature to the crate and try again.").into());
        }

        // Set the profile of the build if it's not already set
        if self.profile.is_none() {
            if self.platform == Some(Platform::Web) {
                self.profile = Some(crate::dioxus_crate::CLIENT_PROFILE_WEB.to_string());
            }
        }

        // If we're building a server and building for web, we need to set the server profile
        // todo(jon): idek if this is right - do we need profiles here?
        if self.server_profile.is_none() {
            self.server_profile = Some(crate::dioxus_crate::SERVER_PROFILE.to_string());
        }

        Ok(())
    }

    /// Get the platform from the build arguments
    pub(crate) fn platform(&self) -> Platform {
        self.platform.expect("Platform was not set")
    }

    pub(crate) fn auto_detect_platform(&self, krate: &DioxusCrate) -> Option<Platform> {
        let features = self.dioxus_features(krate, None);
        todo!()
    }

    // /// Get the features that we need to pass to the build to make `--platform` work properly.
    // ///
    // /// This is because usually folks will lock a platform behind a feature, and `dx serve --platform` will
    // /// will need to pass the feature to the build.
    // ///
    // /// Usually the feature will be `dioxus/<platform>`, but this guarantee it
    // pub(crate) fn features_for_platform(&self, platform: Platform) -> Vec<String> {
    //     todo!()
    // }

    /// Check if dioxus is being built with a particular feature
    fn has_dioxus_feature(&self, krate: &DioxusCrate, filter: &str) -> bool {
        krate.krates.krates_by_name("dioxus").any(|dioxus| {
            krate
                .krates
                .get_enabled_features(dioxus.kid)
                .map(|features| features.contains(filter))
                .unwrap_or_default()
        })
    }

    pub(crate) fn auto_detect_server_feature(&self, krate: &DioxusCrate) -> Option<String> {
        todo!()
    }

    fn dioxus_features(&self, krate: &DioxusCrate, platform: Option<Platform>) -> Vec<String> {
        todo!()
    }

    // fn features_for_platform(&self, platform: Platform) -> Vec<String> {
    //     todo!()
    // }

    // pub(crate) fn auto_detect_client_platform(
    //     &self,
    //     resolved: &DioxusCrate,
    // ) -> (Option<String>, Platform) {
    //     self.find_dioxus_feature(resolved, |platform| {
    //         matches!(
    //             platform.parse::<Platform>(),
    //             Some(Platform::Web) | Some(Platform::Desktop)
    //         )
    //     })
    //     .unwrap_or_else(|| (Some("web".to_string()), Platform::Web))
    // }

    // pub(crate) fn auto_detect_server_feature(&self, resolved: &DioxusCrate) -> Option<String> {
    //     self.find_dioxus_feature(resolved, |platform| matches!(platform, Platform::Server))
    //         .map(|(feature, _)| feature)
    //         .unwrap_or_else(|| Some("server".to_string()))
    // }

    // fn auto_detect_platform(&self, resolved: &DioxusCrate) -> Platform {
    //     self.auto_detect_platform_with_filter(resolved, |_| true).1
    // }

    // fn auto_detect_platform_with_filter(
    //     &self,
    //     resolved: &DioxusCrate,
    //     filter_platform: fn(&Platform) -> bool,
    // ) -> (Option<String>, Platform) {
    //     self.find_dioxus_feature(resolved, filter_platform)
    //         .unwrap_or_else(|| {
    //             let default_platform = resolved.config.application.default_platform;

    //             (Some(default_platform.to_string()), default_platform)
    //         })
    // }

    // fn find_dioxus_feature(
    //     &self,
    //     resolved: &DioxusCrate,
    //     filter_platform: fn(&String) -> bool,
    // ) -> Option<String> {
    //     // First check the enabled features for any renderer enabled
    //     for dioxus in resolved.krates.krates_by_name("dioxus") {
    //         let Some(features) = resolved.krates.get_enabled_features(dioxus.kid) else {
    //             continue;
    //         };

    //         if let Some(platform) = features.iter().filter(filter_platform) {
    //             return Some((None, platform));
    //         }
    //     }

    //     // Then check the features that might get enabled
    //     if let Some(platform) = resolved
    //         .package()
    //         .features
    //         .iter()
    //         .find_map(|(feature, enables)| {
    //             enables
    //                 .iter()
    //                 .find_map(|f| {
    //                     f.strip_prefix("dioxus/")
    //                         .or_else(|| feature.strip_prefix("dep:dioxus/"))
    //                         .and_then(|f| f.parse::<P>().ok())
    //                         .filter(filter_platform)
    //                 })
    //                 .map(|platform| (Some(feature.clone()), platform))
    //         })
    //     {
    //         return Some(platform);
    //     }

    //     None
    // }
}
