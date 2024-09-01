use crate::build::Build;
use crate::config::Platform;
use crate::Result;
use crate::{assets::AssetManifest, dioxus_crate::DioxusCrate};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
pub use platform::TargetPlatform;
use std::path::PathBuf;

mod assets;
mod bundle;
mod cargo;
mod fullstack;
mod handle;
mod platform;
mod prepare_html;
mod progress;
mod web;

pub use progress::{
    BuildMessage, MessageSource, MessageType, Stage, UpdateBuildProgress, UpdateStage,
};

/// An app that's built, bundled, processed, and a handle to its running app, if it exists
///
/// As the build progresses, we'll fill in fields like assets, executable, entitlements, etc
///
/// If the app needs to be bundled, we'll add the bundle info here too
pub struct BuildRequest {
    /// Whether the build is for serving the application
    pub reason: BuildReason,

    /// The configuration for the crate we are building
    pub krate: DioxusCrate,

    /// The target platform for the build
    pub target_platform: TargetPlatform,

    /// The arguments for the build
    pub build_arguments: Build,

    /// The rustc flags to pass to the build
    pub rust_flags: Vec<String>,

    /// The target directory for the build
    pub target_dir: Option<PathBuf>,

    /// The output executable location
    pub executable: Option<PathBuf>,

    /// The assets manifest - starts empty and will be populated as we go
    pub assets: AssetManifest,

    /// The child process of this running app that has yet to be spawned.
    ///
    /// We might need to finangle this into something else
    pub child: Option<tokio::process::Child>,

    /// Status channel to send our progress updates to
    pub progress: UnboundedSender<UpdateBuildProgress>,
}

/// The reason for the build - this will determine how we prep the output
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuildReason {
    Serve,
    Build,
    Bundle,
}

impl BuildRequest {
    pub fn create(
        serve: BuildReason,
        dioxus_crate: &DioxusCrate,
        build_arguments: impl Into<Build>,
        progress: UnboundedSender<UpdateBuildProgress>,
    ) -> crate::Result<Vec<Self>> {
        let build_arguments: Build = build_arguments.into();

        let single_platform = |platform| {
            let req = Self::new_single(
                serve,
                dioxus_crate.clone(),
                build_arguments.clone(),
                progress.clone(),
                platform,
            );
            Ok(vec![req])
        };

        match build_arguments.platform() {
            Platform::Liveview => single_platform(TargetPlatform::Liveview),
            Platform::Web => single_platform(TargetPlatform::Web),
            Platform::Desktop => single_platform(TargetPlatform::Desktop),
            Platform::Mobile => single_platform(TargetPlatform::Mobile),

            Platform::Fullstack => {
                Self::new_fullstack(dioxus_crate.clone(), build_arguments, serve, progress)
            }
        }
    }

    pub(crate) async fn build_all_parallel(
        build_requests: Vec<BuildRequest>,
        mut rx: UnboundedReceiver<UpdateBuildProgress>,
    ) -> Result<Vec<BuildRequest>> {
        let multi_platform_build = build_requests.len() > 1;
        let mut set = tokio::task::JoinSet::new();

        for build_request in build_requests {
            set.spawn(async move { build_request.build().await });
        }

        // Watch the build progress as it comes in
        while let Some(update) = rx.next().await {
            if multi_platform_build {
                let platform = update.platform;
                print!("{platform} build: ");
                update.to_std_out();
            } else {
                update.to_std_out();
            }
        }

        let mut all_results = Vec::new();

        while let Some(result) = set.join_next().await {
            all_results.push(
                result
                    .map_err(|_| crate::Error::Unique("Failed to build project".to_owned()))??,
            );
        }

        Ok(all_results)
    }
}
