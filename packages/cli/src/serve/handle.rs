use crate::Result;
use crate::{builder::Platform, bundler::AppBundle};
use std::{net::SocketAddr, path::PathBuf, process::Stdio};
use tokio::{
    io::AsyncBufReadExt,
    process::{Child, Command},
};
use tokio::{
    io::{BufReader, Lines},
    process::{ChildStderr, ChildStdout},
};
use uuid::Uuid;

/// A handle to a running app
pub(crate) struct AppHandle {
    pub(crate) _id: Uuid,
    pub(crate) app: AppBundle,
    pub(crate) executable: PathBuf,
    pub(crate) child: Option<Child>,
    pub(crate) stdout: Option<Lines<BufReader<ChildStdout>>>,
    pub(crate) stderr: Option<Lines<BufReader<ChildStderr>>>,
}

impl AppHandle {
    pub async fn start(
        app: AppBundle,
        devserver_ip: SocketAddr,
        fullstack_address: Option<SocketAddr>,
    ) -> Result<Self> {
        let platform = app.build.platform();
        let ip = devserver_ip.to_string();

        if platform == Platform::Server {
            tracing::trace!(
                "Proxying fullstack server from port {:?}",
                fullstack_address
            );
        }

        let work_dir = std::env::temp_dir();
        let executable = app.finish(work_dir).await?;

        let mut handle = AppHandle {
            app,
            executable,
            _id: Uuid::new_v4(),
            child: None,
            stderr: None,
            stdout: None,
        };

        // open the exe with some arguments/envvars/etc
        // we're going to try and configure this binary from the environment, if we can
        //
        // web can't be configured like this, so instead, we'll need to plumb a meta tag into the
        // index.html during dev
        match handle.app.build.platform() {
            Platform::Desktop | Platform::Server | Platform::Liveview => {
                let mut cmd = Command::new(handle.executable.clone());
                cmd.env(
                    dioxus_runtime_config::FULLSTACK_ADDRESS_ENV,
                    fullstack_address
                        .as_ref()
                        .map(|addr| addr.to_string())
                        .unwrap_or_else(|| "127.0.0.1:8080".to_string()),
                )
                .env(
                    dioxus_runtime_config::IOS_DEVSERVER_ADDR_ENV,
                    format!("ws://{}/_dioxus", ip),
                )
                .env(
                    dioxus_runtime_config::DEVSERVER_RAW_ADDR_ENV,
                    format!("ws://{}/_dioxus", ip),
                )
                .env("CARGO_MANIFEST_DIR", handle.app.build.krate.crate_dir())
                .env(
                    "SIMCTL_CHILD_CARGO_MANIFEST_DIR",
                    handle.app.build.krate.crate_dir(),
                )
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .kill_on_drop(true);

                let mut child = cmd.spawn()?;
                let stdout = BufReader::new(child.stdout.take().unwrap());
                let stderr = BufReader::new(child.stderr.take().unwrap());
                handle.stdout = Some(stdout.lines());
                handle.stderr = Some(stderr.lines());
                handle.child = Some(child);
            }
            Platform::Web => {}
            Platform::Ios => {}
            Platform::Android => {}
        }

        Ok(handle)
    }
    /// Update an asset in the running apps
    ///
    /// Might need to upload the asset to the simulator or overwrite it within the bundle
    ///
    /// Returns the name of the asset in the bundle if it exists
    pub(crate) fn hotreload_asset(&self, path: &PathBuf) -> Option<PathBuf> {
        let resource = self.app.assets.assets.get(path).cloned()?;

        _ = self
            .app
            .assets
            .copy_asset_to(&self.app.asset_dir(), path, false, false);

        Some(resource.bundled.into())
    }

    #[allow(unused)]
    fn open_bundled_ios_app(&self, build: &AppBundle) -> std::io::Result<Option<Child>> {
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
