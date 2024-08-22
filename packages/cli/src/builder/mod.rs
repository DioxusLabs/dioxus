use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use crate::{build::Build, config};
use crate::{cli::serve::ServeArguments, config::Platform};
use futures_util::stream::select_all;
use futures_util::StreamExt;
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
    pub dioxus_crate: DioxusCrate,

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
    ) -> crate::Result<Vec<Self>> {
        let build_arguments = build_arguments.into();
        let platform = build_arguments.platform();
        let single_platform = |platform| {
            let dioxus_crate = dioxus_crate.clone();

            let request = Self {
                reason: serve,
                dioxus_crate,
                build_arguments: build_arguments.clone(),
                target_platform: platform,
                rust_flags: Default::default(),
                target_dir: Default::default(),
                executable: Default::default(),
            };

            vec![request]
        };

        Ok(match platform {
            Platform::Liveview => single_platform(TargetPlatform::Liveview),
            Platform::Web => single_platform(TargetPlatform::Web),
            Platform::Desktop => single_platform(TargetPlatform::Desktop),
            Platform::StaticGeneration | Platform::Fullstack => {
                Self::new_fullstack(dioxus_crate.clone(), build_arguments, serve)?
            }
            _ => unimplemented!("Unknown platform: {platform:?}"),
        })
    }

    pub(crate) async fn build_all_parallel(
        build_requests: Vec<BuildRequest>,
    ) -> Result<Vec<BuildRequest>> {
        let multi_platform_build = build_requests.len() > 1;
        let mut build_progress = Vec::new();
        let mut set = tokio::task::JoinSet::new();
        for build_request in build_requests {
            let (tx, rx) = futures_channel::mpsc::unbounded();
            build_progress.push((build_request.build_arguments.platform(), rx));
            set.spawn(async move { build_request.build(tx).await });
        }

        // Watch the build progress as it comes in
        loop {
            let mut next = select_all(
                build_progress
                    .iter_mut()
                    .map(|(platform, rx)| rx.map(move |update| (*platform, update))),
            );
            match next.next().await {
                Some((platform, update)) => {
                    if multi_platform_build {
                        print!("{platform} build: ");
                        update.to_std_out();
                    } else {
                        update.to_std_out();
                    }
                }
                None => {
                    break;
                }
            }
        }

        let mut all_results = Vec::new();

        while let Some(result) = set.join_next().await {
            let result = result
                .map_err(|_| crate::Error::Unique("Failed to build project".to_owned()))??;
            all_results.push(result);
        }

        Ok(all_results)
    }

    /// Check if the build is targeting the web platform
    pub fn targeting_web(&self) -> bool {
        self.target_platform == TargetPlatform::Web
    }

    /// Open the executable if this is a native build
    pub fn open(
        &self,
        config: &DioxusCrate,
        serve: &ServeArguments,
        fullstack_address: Option<SocketAddr>,
        workspace: &Path,
    ) -> std::io::Result<Option<Child>> {
        if self.target_platform == TargetPlatform::Web {
            return Ok(None);
        }

        if self.target_platform == TargetPlatform::Server {
            tracing::trace!("Proxying fullstack server from port {fullstack_address:?}");
        }

        //
        // open the exe with some arguments/envvars/etc
        // we're going to try and configure this binary from the environment, if we can
        //
        // web can't be configured like this, so instead, we'll need to plumb a meta tag into the
        // index.html during dev
        //
        let res = Command::new(
            self.executable
                .as_deref()
                .expect("executable should be built if we're trying to open it")
                .canonicalize()?,
        )
        .env(
            dioxus_runtime_config::FULLSTACK_ADDRESS_ENV,
            fullstack_address
                .as_ref()
                .map(|addr| addr.to_string())
                .unwrap_or_else(|| "127.0.0.1:8080".to_string()),
        )
        .env(
            dioxus_runtime_config::DEVSERVER_ADDR_ENV,
            serve.address.address().to_string(),
        )
        .env(
            dioxus_runtime_config::IOS_DEVSERVER_ADDR_ENV,
            serve.address.address().to_string(),
        )
        .env("CARGO_MANIFEST_DIR", config.crate_dir())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .current_dir(workspace)
        .spawn()?;

        Ok(Some(res))
    }
}
