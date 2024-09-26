use crate::{bundler::AppBundle, Platform};
use crate::{Result, TraceSrc};
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
///
/// Also includes a handle to its server if it exists
pub(crate) struct AppHandle {
    pub(crate) _id: Uuid,
    pub(crate) build: AppBundle,

    pub(crate) platform: Platform,
    pub(crate) app_child: Option<Child>,
    pub(crate) server_child: Option<Child>,

    /// The virtual directory that assets will be served from
    pub(crate) runtime_asst_dir: Option<PathBuf>,

    pub(crate) app_stdout: Option<Lines<BufReader<ChildStdout>>>,
    pub(crate) app_stderr: Option<Lines<BufReader<ChildStderr>>>,
    pub(crate) server_stdout: Option<Lines<BufReader<ChildStdout>>>,
    pub(crate) server_stderr: Option<Lines<BufReader<ChildStderr>>>,
}

impl AppHandle {
    pub async fn start(
        app: AppBundle,
        devserver_ip: SocketAddr,
        fullstack_address: Option<SocketAddr>,
    ) -> Result<Self> {
        let mut handle = AppHandle {
            _id: Uuid::new_v4(),
            platform: app.build.build.platform(),
            build: app,
            runtime_asst_dir: None,
            app_child: None,
            app_stderr: None,
            app_stdout: None,
            server_child: None,
            server_stdout: None,
            server_stderr: None,
        };

        handle.open(devserver_ip, fullstack_address).await?;

        Ok(handle)
    }

    pub(crate) async fn open(
        &mut self,
        devserver_ip: SocketAddr,
        fullstack_address: Option<SocketAddr>,
    ) -> Result<()> {
        let platform = self.platform;

        if platform == Platform::Server || self.build.build.build.fullstack {
            tracing::debug!(
                "Proxying fullstack server from port {:?}",
                fullstack_address
            );
        }

        // Set the env vars that the clients will expect
        // These need to be stable within a release version (ie 0.6.0)
        let mut envs = vec![
            ("DIOXUS_CLI_ENABLED", "true".to_string()),
            (
                "CARGO_MANIFEST_DIR",
                self.build.build.krate.crate_dir().display().to_string(),
            ),
            (
                "SIMCTL_CHILD_CARGO_MANIFEST_DIR",
                self.build.build.krate.crate_dir().display().to_string(),
            ),
            (
                dioxus_cli_config::DEVSERVER_RAW_ADDR_ENV,
                devserver_ip.to_string(),
            ),
            // (
            //     dioxus_cli_config::ALWAYS_ON_TOP_ENV,
            //     serve.always_on_top.unwrap_or(true).to_string(),
            // ),
            // cmd.env(dioxus_cli_config::APP_TITLE_ENV, app_title);
            // cmd.env(dioxus_cli_config::OUT_DIR, out_dir.display().to_string());
        ];

        if let Some(addr) = fullstack_address {
            envs.push((dioxus_cli_config::SERVER_IP_ENV, addr.ip().to_string()));
            envs.push((dioxus_cli_config::SERVER_PORT_ENV, addr.port().to_string()));
        };

        // Launch the server if we have one
        if let Some(server) = self.build.server() {
            tracing::debug!("Launching server: {server:?}");
            let mut cmd = Command::new(server);

            cmd.envs(envs.clone())
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .kill_on_drop(true);

            let mut child = cmd.spawn()?;
            let stdout = BufReader::new(child.stdout.take().unwrap());
            let stderr = BufReader::new(child.stderr.take().unwrap());
            self.server_stdout = Some(stdout.lines());
            self.server_stderr = Some(stderr.lines());
            self.server_child = Some(child);
        }

        match self.platform {
            Platform::Web => {
                // tracing::info!(dx_src = ?TraceSrc::Dev, "Serving web app on http://{} 🎉", ip);
            }
            Platform::Desktop => {
                // tracing::info!(dx_src = ?TraceSrc::Dev, "Launching desktop app 🎉");
                // tracing::debug!(dx_src = ?TraceSrc::Dev, "Desktop app location: {:?}", self.build_dir.display());
            }
            Platform::Server => {
                if let Some(fullstack_address) = fullstack_address {
                    // tracing::info!(
                    //     dx_src = ?TraceSrc::Dev,
                    //     "Launching fullstack server on http://{:?} 🎉",
                    //     fullstack_address
                    // );
                }
            }
            Platform::Ios => {}
            Platform::Android => {}
            Platform::Liveview => {
                if let Some(fullstack_address) = fullstack_address {
                    // tracing::info!(
                    //     dx_src = ?TraceSrc::Dev,
                    //     "Launching liveview server on http://{:?} 🎉",
                    //     fullstack_address
                    // );
                }
            }
        }

        let running_process = match self.build.build.build.platform() {
            Platform::Desktop => Some(self.open_mac_desktop(envs)?),
            Platform::Web => {
                // web can't be configured like this, so instead, we'll need to plumb a meta tag into the
                // index.html during dev
                self.open_web(envs);
                None
            }
            Platform::Ios => Some(self.open_ios_sim(envs).await?),
            Platform::Android => todo!(),
            Platform::Liveview => todo!(),
            Platform::Server => todo!(),
        };

        // If we have a running process, we need to attach to it and wait for its outputs
        if let Some(mut child) = running_process {
            let stdout = BufReader::new(child.stdout.take().unwrap());
            let stderr = BufReader::new(child.stderr.take().unwrap());
            self.app_stdout = Some(stdout.lines());
            self.app_stderr = Some(stderr.lines());
            self.app_child = Some(child);
        }

        Ok(())
    }

    fn open_mac_desktop(&mut self, envs: Vec<(&str, String)>) -> Result<Child> {
        Ok(Command::new(self.build.main_exe())
            .envs(envs)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?)
    }

    pub(crate) fn hotreload_bundled_asset(
        &self,
        absolute_changed_file: &PathBuf,
    ) -> Option<PathBuf> {
        let mut bundled_name = None;

        let asset_dir = match self.runtime_asst_dir.as_ref() {
            Some(dir) => dir.to_path_buf().join("assets/"),
            None => self.build.asset_dir(),
        };

        tracing::debug!("Hotreloading asset {absolute_changed_file:?} in target {asset_dir:?}");

        // If the asset shares the same name in the bundle, reload that
        let legacy_asset_dir = self.build.build.krate.legacy_asset_dir();
        if absolute_changed_file.starts_with(&legacy_asset_dir) {
            tracing::debug!("Hotreloading legacy asset {absolute_changed_file:?}");
            let trimmed = absolute_changed_file
                .strip_prefix(legacy_asset_dir)
                .unwrap();
            let res = std::fs::copy(absolute_changed_file, asset_dir.join(trimmed));
            bundled_name = Some(trimmed.to_path_buf());
            if let Err(e) = res {
                tracing::debug!("Failed to hotreload legacy asset {e}");
            }
        }

        // The asset might've been renamed thanks to the manifest, let's attempt to reload that too
        let resource = self
            .build
            .app_assets
            .assets
            .get(absolute_changed_file)
            .cloned();

        if let Some(resource) = resource {
            let res = std::fs::copy(absolute_changed_file, asset_dir.join(&resource.bundled));
            bundled_name = Some(PathBuf::from(resource.bundled));

            if let Err(e) = res {
                tracing::debug!("Failed to hotreload asset {e}");
            }
        }

        // Now let's modify the running app, if we need to
        // Every platform does this differently in quriky ways
        match self.build.build.build.platform() {
            // Nothing to do - editing the build dir is enough since we serve from there anyways
            Platform::Web => {}

            // Nothing to do - we serve from the .app dir which is executable anyways
            Platform::Desktop => {}

            // These share .appimage semantics, so modifying the build dir is enough
            Platform::Liveview => {}
            Platform::Server => {}

            // todo: I think we need to modify the simulator mount folder
            Platform::Ios => {
                // the simulator will mount the app to somewhere in the CoreSimulator dir
                // we could try to communicate this back the the host...
                // /Users/jonkelley/Library/Developer/CoreSimulator/Devices/83AE3067-987F-4F85-AE3D-7079EF48C967/data/Containers/Bundle/Application/6C4F0EDF-291E-4EDC-ABCF-B4225762073A/DioxusApp.app
            }

            // todo: I think we need to modify the simulator mount folder / and/or adb a new file in
            Platform::Android => todo!(),
        };

        // Now we can return the bundled asset name to send to the hotreload engine
        bundled_name
    }

    async fn open_ios_sim(&mut self, envs: Vec<(&str, String)>) -> Result<Child> {
        // Install the app
        // xcrun simctl install booted DioxusApp.app
        tracing::debug!("Installing app to simulator {:?}", self.build.app_root());
        let res = Command::new("xcrun")
            .arg("simctl")
            .arg("install")
            .arg("booted")
            .arg(self.build.app_root())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .await?;

        tracing::debug!("Installed app to simulator with exit code: {res:?}");

        // env = {
        //  SIMCTL_CHILD_DIOXUS_DEVSERVER_ADDR="ws://0.0.0.0:8080/_dioxus",
        //  SIMCTL_CHILD_CARGO_MANIFEST_DIR="/Users/jonkelley/Development/Tinkering/ios-binary"
        // }
        // args = ["simctl", "launch", "--console", "booted", "com.dioxuslabs"]
        // command = "xcrun"
        // dependencies = ["build_ios_sim", "install_ios_sim"]

        // Remap the envs to the correct simctl env vars
        let envs = envs
            .iter()
            .map(|(k, v)| (format!("SIMCTL_CHILD_{k}"), v.clone()));

        let child = Command::new("xcrun")
            .arg("simctl")
            .arg("launch")
            .arg("--console")
            .arg("booted")
            .arg("com.dioxuslabs")
            .envs(envs)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        tracing::debug!("Launched app on simulator with exit code: {child:?}");

        Ok(child)

        // command = "xcrun"
        // args = [
        // "simctl",
        // "install",
        // "booted",
        // "target/aarch64-apple-ios-sim/debug/bundle/ios/DioxusApp.app",
        // ] \

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

        // // Install the app
        // let mut cmd = Command::new("xcrun");
        // cmd.arg("simctl")
        //     .arg("launch")
        //     .arg("--console")
        //     .arg("booted")
        //     .arg(self.build_dir.clone());
        // let mut res = cmd.spawn()?;
        // let res = res.wait().await?;
    }

    fn open_web(&self, envs: Vec<(&str, String)>) {
        // let start_browser = args.open.unwrap_or_default();
        // let base_path = cfg.dioxus_config.web.app.base_path.clone();
        // let platform = args.platform();
        // // Open the browser
        // if start_browser && platform != Platform::Desktop {
        //     open_browser(base_path, addr, rustls.is_some());
        // }
        // // let protocol = if https { "https" } else { "http" };
        // let base_path = match base_path.as_deref() {
        //     Some(base_path) => format!("/{}", base_path.trim_matches('/')),
        //     None => "".to_owned(),
        // };
        // _ = open::that(format!("{protocol}://{address}{base_path}"));
        // _ = open::that(format!("http://{devserver_ip}"));
    }
}
