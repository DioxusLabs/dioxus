use super::profiles::*;
use super::BuildResult;
use crate::build::BuildArgs;
use crate::builder::Platform;
use crate::Result;
use crate::{assets::AssetManifest, dioxus_crate::DioxusCrate};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use std::path::PathBuf;

pub use super::progress::{
    BuildMessage, MessageSource, MessageType, Stage, UpdateBuildProgress, UpdateStage,
};

/// An app that's built, bundled, processed, and a handle to its running app, if it exists
///
/// As the build progresses, we'll fill in fields like assets, executable, entitlements, etc
///
/// If the app needs to be bundled, we'll add the bundle info here too
pub struct BuildRequest {
    /// The configuration for the crate we are building
    pub krate: DioxusCrate,

    /// The arguments for the build
    pub build: BuildArgs,

    /// The rustc flags to pass to the build
    pub rust_flags: Vec<String>,

    /// The target directory for the build
    pub custom_target_dir: Option<PathBuf>,

    /// Status channel to send our progress updates to
    pub progress: UnboundedSender<UpdateBuildProgress>,
}

impl BuildRequest {
    pub fn new(
        krate: DioxusCrate,
        build_arguments: impl Into<BuildArgs>,
        progress: UnboundedSender<UpdateBuildProgress>,
    ) -> Self {
        let build: BuildArgs = build_arguments.into();

        Self {
            progress,
            build,
            krate,
            custom_target_dir: Default::default(),
            rust_flags: Default::default(),
        }
    }

    fn new_with_target_directory_rust_flags_and_features(
        krate: &DioxusCrate,
        build: &BuildArgs,
        feature: Option<String>,
        target_platform: Platform,
        progress: UnboundedSender<UpdateBuildProgress>,
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

    pub fn new_server(
        krate: &DioxusCrate,
        mut build: BuildArgs,
        progress: UnboundedSender<UpdateBuildProgress>,
    ) -> Self {
        if build.profile.is_none() {
            build.profile = Some(CLIENT_PROFILE.to_string());
        }
        let client_feature = build.auto_detect_server_feature(krate);
        Self::new_with_target_directory_rust_flags_and_features(
            krate,
            &build,
            build.target_args.server_feature.clone().or(client_feature),
            Platform::Server,
            progress,
        )
    }

    pub fn new_client(
        krate: &DioxusCrate,
        mut build: BuildArgs,
        progress: UnboundedSender<UpdateBuildProgress>,
    ) -> Self {
        if build.profile.is_none() {
            build.profile = Some(SERVER_PROFILE.to_string());
        }
        let (client_feature, client_platform) = build.auto_detect_client_platform(krate);
        Self::new_with_target_directory_rust_flags_and_features(
            krate,
            &build,
            build.target_args.client_feature.clone().or(client_feature),
            client_platform,
            progress,
        )
    }

    pub(crate) async fn build_all_parallel(
        build_requests: Vec<BuildRequest>,
        mut rx: UnboundedReceiver<UpdateBuildProgress>,
    ) -> Result<Vec<BuildRequest>> {
        let multi_platform_build = build_requests.len() > 1;
        // let mut set = tokio::task::JoinSet::new();

        // for build_request in build_requests {
        //     set.spawn(async move { build_request.build().await });
        // }

        // // Watch the build progress as it comes in
        // while let Some(update) = rx.next().await {
        //     if multi_platform_build {
        //         let platform = update.platform;
        //         print!("{platform} build: ");
        //         update.to_std_out();
        //     } else {
        //         update.to_std_out();
        //     }
        // }

        todo!()

        // let mut all_results = Vec::new();

        // while let Some(result) = set.join_next().await {
        //     all_results.push(
        //         result
        //             .map_err(|_| crate::Error::Unique("Failed to build project".to_owned()))??,
        //     );
        // }

        // Ok(all_results)
    }

    pub(crate) fn new_single(
        krate: DioxusCrate,
        arguments: BuildArgs,
        progress: UnboundedSender<UpdateBuildProgress>,
    ) -> Self {
        Self {
            progress,
            krate,
            build: arguments,
            custom_target_dir: Default::default(),
            rust_flags: Default::default(),
        }
    }

    /// Get the platform for this build
    pub fn platform(&self) -> Platform {
        self.build
            .platform
            .unwrap_or_else(|| self.krate.dioxus_config.application.default_platform)
    }
}
