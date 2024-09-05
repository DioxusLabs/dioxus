use super::ServeUpdate;
use crate::{
    builder::{AppBundle, BuildUpdate, Platform},
    cli::serve::ServeArgs,
    DioxusCrate, Result,
};
use manganis_core::ResourceAsset;
use std::{collections::HashMap, fs, net::SocketAddr, path::PathBuf, process::Stdio};
use tokio::process::Child;
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{ChildStderr, ChildStdout, Command},
};
use uuid::Uuid;

pub struct AppRunner {
    /// Ongoing apps running in place
    ///
    /// They might be actively being being, running, or have exited.
    ///
    /// When a new full rebuild occurs, we will keep these requests here
    pub running: HashMap<Platform, AppHandle>,
}

/// A handle to a running app
pub struct AppHandle {
    pub app: AppBundle,
    pub executable: PathBuf,
    pub id: Uuid,
    pub child: Option<Child>,
    // pub stdout: Lines<BufReader<ChildStdout>>,
    // pub stderr: Lines<BufReader<ChildStderr>>,
    // pub stdout_line: String,
    // pub stderr_line: String,
}

impl AppRunner {
    pub fn start(serve: &ServeArgs, config: &DioxusCrate) -> Self {
        Self {
            running: Default::default(),
        }
    }

    pub async fn wait(&mut self) -> ServeUpdate {
        // // Exits and stdout/stderr
        //         let processes = self.running.iter_mut().filter_map(|(target, request)| {
        //             let Some(child) = request.child else {
        //                 return None;
        //             };

        //             Some(Box::pin(async move {
        //                 //
        //                 (*target, child.wait().await)
        //             }))
        //         });

        //             ((target, exit_status), _, _) = futures_util::future::select_all(processes) => {
        //                 BuildUpdate::ProcessExited { status: exit_status, target_platform: target }
        //             }

        // let has_running_apps = !self.running_apps.is_empty();
        // let next_stdout = self.running_apps.values_mut().map(|app| {
        //     let future = async move {
        //         let (stdout, stderr) = match &mut app.output {
        //             Some(out) => (
        //                 ok_and_some(out.stdout.next_line()),
        //                 ok_and_some(out.stderr.next_line()),
        //             ),
        //             None => return futures_util::future::pending().await,
        //         };

        //         tokio::select! {
        //             line = stdout => (app.result.target_platform, Some(line), None),
        //             line = stderr => (app.result.target_platform, None, Some(line)),
        //         }
        //     };
        //     Box::pin(future)
        // });

        // let next_stdout = async {
        //     if has_running_apps {
        //         select_all(next_stdout).await.0
        //     } else {
        //         futures_util::future::pending().await
        //     }
        // };
        //     (platform, stdout, stderr) = next_stdout => {
        //         if let Some(stdout) = stdout {
        //             self.push_stdout(platform, stdout);
        //         }
        //         if let Some(stderr) = stderr {
        //             self.push_stderr(platform, stderr);
        //         }
        //     },

        futures_util::future::pending().await
    }

    /// Finally "bundle" this app and return a handle to it
    pub async fn open(&mut self, app: AppBundle, fullstack_addr: SocketAddr) -> Result<&AppHandle> {
        let platform = app.build.platform();

        if platform == Platform::Server {
            tracing::trace!("Proxying fullstack server from port {:?}", fullstack_addr);
        }

        let work_dir = std::env::temp_dir();
        let executable = app.finish(work_dir).await?;

        //         stdout: BufReader::new(stdout).lines(),
        //         stderr: BufReader::new(stderr).lines(),
        //         stdout_line: String::new(),
        //         stderr_line: String::new(),

        let handle = AppHandle {
            app,
            executable,
            child: None,
            // stdout: BufReader::new(stdout).lines(),
            // stderr: BufReader::new(stderr).lines(),
            // stdout_line: String::new(),
            // stderr_line: String::new(),
            id: Uuid::new_v4(),
        };

        if let Some(previous) = self.running.insert(platform, handle) {
            // close the old app, gracefully, hopefully
        }

        Ok(self.running.get(&platform).unwrap())

        // // First, we need to "install" the app
        // let exe = build.finish(work_dir).await?;
        // let mut open = match build.build.platform() {
        //     // Run `dx http-server` to serve the app
        //     Platform::Web => todo!(),

        //     // Open up the .ipa for the .app
        //     Platform::Ios => todo!(),

        //     Platform::Desktop => Command::new("open"),
        //     Platform::Android => todo!("Android not supported yet"),
        //     Platform::Server | Platform::Liveview => Command::new(exe.display().to_string()),
        // };

        // open the exe with some arguments/envvars/etc
        // we're going to try and configure this binary from the environment, if we can
        //
        // web can't be configured like this, so instead, we'll need to plumb a meta tag into the
        // index.html during dev
        // let _ = open
        //     .env(
        //         dioxus_runtime_config::FULLSTACK_ADDRESS_ENV,
        //         self.fullstack_address()
        //             .as_ref()
        //             .map(|addr| addr.to_string())
        //             .unwrap_or_else(|| "127.0.0.1:8080".to_string()),
        //     )
        //     .env(
        //         dioxus_runtime_config::IOS_DEVSERVER_ADDR_ENV,
        //         format!("ws://{}/_dioxus", ip),
        //     )
        //     .env(
        //         dioxus_runtime_config::DEVSERVER_RAW_ADDR_ENV,
        //         format!("ws://{}/_dioxus", ip),
        //     )
        //     .env("CARGO_MANIFEST_DIR", build.build.krate.crate_dir())
        //     .stderr(Stdio::piped())
        //     .stdout(Stdio::piped())
        //     .kill_on_drop(true);

        // todo!()
    }

    fn install_app(&self, build: &AppBundle) -> Result<()> {
        todo!()
    }

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

impl AppHandle {
    /// Update an asset in the running apps
    ///
    /// Might need to upload the asset to the simulator or overwrite it within the bundle
    ///
    /// Returns the name of the asset in the bundle if it exists
    pub fn update_asset(&self, path: &PathBuf) -> Option<PathBuf> {
        let resource = self.app.assets.assets.get(path).cloned()?;

        self.app
            .assets
            .copy_asset_to(&self.app.asset_dir(), path, false, false);

        Some(resource.bundled.into())
    }
}
