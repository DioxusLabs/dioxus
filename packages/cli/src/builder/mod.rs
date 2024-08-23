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

    /// Open the executable if this is a native build
    pub fn open(
        &self,
        serve: &ServeArguments,
        fullstack_address: Option<SocketAddr>,
    ) -> std::io::Result<Option<Child>> {
        if self.target_platform == TargetPlatform::Server {
            tracing::trace!("Proxying fullstack server from port {fullstack_address:?}");
        }

        match self.target_platform {
            TargetPlatform::Web => Ok(None),
            TargetPlatform::Mobile => self.open_bundled_ios_app(serve),
            TargetPlatform::Desktop | TargetPlatform::Server | TargetPlatform::Liveview => {
                self.open_unbundled_native_app(serve, fullstack_address)
            }
        }
    }

    fn open_unbundled_native_app(
        &self,
        serve: &ServeArguments,
        fullstack_address: Option<SocketAddr>,
    ) -> std::io::Result<Option<Child>> {
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
            dioxus_runtime_config::RAW_DEVSERVER_ADDR_ENV,
            serve.address.address().to_string(),
        )
        .env(
            dioxus_runtime_config::IOS_DEVSERVER_ADDR_ENV,
            serve.address.address().to_string(),
        )
        .env("CARGO_MANIFEST_DIR", self.krate.crate_dir())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .current_dir(self.krate.workspace_dir())
        .spawn()?;

        Ok(Some(res))
    }

    fn open_bundled_ios_app(&self, serve: &ServeArguments) -> std::io::Result<Option<Child>> {
        // command = "xcrun"
        // args = [
        // "simctl",
        // "install",
        // "booted",
        // "target/aarch64-apple-ios-sim/debug/bundle/ios/DioxusApp.app",
        // ]

        // [tasks.run_ios_sim]
        // args = ["simctl", "launch", "--console", "booted", "com.dioxuslabs"]
        // command = "xcrun"
        // dependencies = ["build_ios_sim", "install_ios_sim"]

        // [tasks.serve-sim]
        // dependencies = ["build_ios_sim", "install_ios_sim", "run_ios_sim"]

        // APP_PATH="target/aarch64-apple-ios/debug/bundle/ios/DioxusApp.app"

        // # get the device id by jq-ing the json of the device list
        // xcrun devicectl list devices --json-output target/deviceid.json
        // DEVICE_UUID=$(jq -r '.result.devices[0].identifier' target/deviceid.json)

        // xcrun devicectl device install app --device "${DEVICE_UUID}" "${APP_PATH}" --json-output target/xcrun.json

        // # get the installation url by jq-ing the json of the device install
        // INSTALLATION_URL=$(jq -r '.result.installedApplications[0].installationURL' target/xcrun.json)

        // # launch the app
        // # todo: we can just background it immediately and then pick it up for loading its logs
        // xcrun devicectl device process launch --device "${DEVICE_UUID}" "${INSTALLATION_URL}"

        // # # launch the app and put it in background
        // # xcrun devicectl device process launch --no-activate --verbose --device "${DEVICE_UUID}" "${INSTALLATION_URL}" --json-output "${XCRUN_DEVICE_PROCESS_LAUNCH_LOG_DIR}"

        // # # Extract background PID of status app
        // # STATUS_PID=$(jq -r '.result.process.processIdentifier' "${XCRUN_DEVICE_PROCESS_LAUNCH_LOG_DIR}")
        // # "${GIT_ROOT}/scripts/wait-for-metro-port.sh"  2>&1

        // # # now that metro is ready, resume the app from background
        // # xcrun devicectl device process resume --device "${DEVICE_UUID}" --pid "${STATUS_PID}" > "${XCRUN_DEVICE_PROCESS_RESUME_LOG_DIR}" 2>&1
        todo!("Open mobile apps")
    }
}
