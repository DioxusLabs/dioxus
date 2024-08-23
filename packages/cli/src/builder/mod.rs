use crate::Result;
use crate::{assets::AssetManifest, dioxus_crate::DioxusCrate};
use crate::{build::Build, config};
use crate::{cli::serve::ServeArguments, config::Platform};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::stream::select_all;
use futures_util::StreamExt;
pub use platform::TargetArch;
pub use platform::TargetPlatform;
use std::{net::SocketAddr, path::Path};
use std::{path::PathBuf, process::Stdio};
use tokio::process::{Child, Command};

mod assets;
mod cargo;
mod fullstack;
mod platform;
mod prepare_html;
mod progress;
mod web;

pub use progress::{
    BuildMessage, MessageSource, MessageType, Stage, UpdateBuildProgress, UpdateStage,
};

/// A request for a project to be built
///
/// As the build progresses, we'll fill in fields like assets, executable, entitlements, etc
///
/// This request will be then passed to the bundler to create a final bundled app
#[derive(Clone)]
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
        let platform = build_arguments.platform();
        let single_platform = |platform| {
            let dioxus_crate = dioxus_crate.clone();

            let request = Self {
                reason: serve,
                krate: dioxus_crate,
                build_arguments: build_arguments.clone(),
                target_platform: platform,
                rust_flags: Default::default(),
                target_dir: Default::default(),
                executable: Default::default(),
                assets: Default::default(),
                progress: progress.clone(),
            };

            vec![request]
        };

        Ok(match platform {
            Platform::Liveview => single_platform(TargetPlatform::Liveview),
            Platform::Web => single_platform(TargetPlatform::Web),
            Platform::Desktop => single_platform(TargetPlatform::Desktop),
            Platform::StaticGeneration | Platform::Fullstack => {
                Self::new_fullstack(dioxus_crate.clone(), build_arguments, serve, progress)?
            }
            _ => unimplemented!("Unknown platform: {platform:?}"),
        })
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

    /// Check if the build is targeting the web platform
    pub fn targeting_web(&self) -> bool {
        self.target_platform == TargetPlatform::Web
    }
}
