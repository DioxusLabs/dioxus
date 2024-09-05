use super::progress::ProgressTx;
use super::{platform, profiles::*};
use crate::build::BuildArgs;
use crate::builder::Platform;
use crate::dioxus_crate::DioxusCrate;
use std::path::PathBuf;

/// An app that's built, bundled, processed, and a handle to its running app, if it exists
///
/// As the build progresses, we'll fill in fields like assets, executable, entitlements, etc
///
/// If the app needs to be bundled, we'll add the bundle info here too
pub(crate) struct BuildRequest {
    /// The configuration for the crate we are building
    pub(crate) krate: DioxusCrate,

    /// The arguments for the build
    pub(crate) build: BuildArgs,

    /// The rustc flags to pass to the build
    pub(crate) rust_flags: Vec<String>,

    /// The target directory for the build
    pub(crate) custom_target_dir: Option<PathBuf>,

    /// Status channel to send our progress updates to
    pub(crate) progress: ProgressTx,
}

impl BuildRequest {
    pub(crate) fn new_client(
        krate: &DioxusCrate,
        mut build: BuildArgs,
        progress: ProgressTx,
    ) -> Self {
        if build.profile.is_none() {
            build.profile = Some(CLIENT_PROFILE.to_string());
        }

        let (client_feature, client_platform) = build.auto_detect_client_platform(krate);

        let client_feature = match build.platform {
            Some(platform::Platform::Ios) => Some("mobile".to_string()),
            Some(platform::Platform::Android) => Some("android".to_string()),
            Some(plat) => Some(plat.to_string()),
            None => client_feature,
        };

        let features = build.target_args.client_feature.clone().or(client_feature);

        tracing::info!("Client feature: {features:?}");

        let mut build = Self::new_with_target_directory_rust_flags_and_features(
            krate, &build, features, progress,
        );

        build.build.platform = build.build.platform.or(Some(client_platform));
        build
    }

    pub(crate) fn new_server(
        krate: &DioxusCrate,
        mut build: BuildArgs,
        progress: ProgressTx,
    ) -> Self {
        if build.profile.is_none() {
            build.profile = Some(SERVER_PROFILE.to_string());
        }

        let client_feature = build.auto_detect_server_feature(krate);
        let features = build.target_args.server_feature.clone().or(client_feature);
        tracing::info!("Server feature: {features:?}");
        let mut build = Self::new_with_target_directory_rust_flags_and_features(
            krate, &build, features, progress,
        );

        build.build.platform = Some(platform::Platform::Server);
        build
    }

    fn new_with_target_directory_rust_flags_and_features(
        krate: &DioxusCrate,
        build: &BuildArgs,
        feature: Option<String>,
        progress: ProgressTx,
    ) -> Self {
        let config = krate.clone();
        let mut build = build.clone();

        // Add the server feature to the features we pass to the build
        if let Some(feature) = feature {
            build.target_args.features.push(feature);
        }

        // Add the server flags to the build arguments
        Self {
            build: build.clone(),
            krate: config,
            rust_flags: Default::default(),
            custom_target_dir: None,
            progress,
        }
    }

    /// Get the platform for this build
    pub(crate) fn platform(&self) -> Platform {
        self.build
            .platform
            .unwrap_or_else(|| self.krate.dioxus_config.application.default_platform)
    }

    /// The final output name of the app, primarly to be used when bundled
    ///
    /// Needs to be very disambiguated
    /// Eg: my-app-web-macos-x86_64.app
    /// {app_name}-{platform}-{arch}
    ///
    /// Does not include the extension
    pub(crate) fn app_name(&self) -> String {
        match self.platform() {
            Platform::Web => "web".to_string(),
            Platform::Desktop => todo!(),
            Platform::Ios => todo!(),
            Platform::Server => "server".to_string(),
            Platform::Android => todo!(),
            Platform::Liveview => todo!(),
        }
    }
}
