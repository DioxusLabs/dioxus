use crate::{AppBundle, Platform, Result};
use anyhow::Context;
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{Child, ChildStderr, ChildStdout, Command},
};

/// A handle to a running app.
///
/// Also includes a handle to its server if it exists.
/// The actual child processes might not be present (web) or running (died/killed).
///
/// The purpose of this struct is to accumulate state about the running app and its server, like
/// any runtime information needed to hotreload the app or send it messages.
///
/// We might want to bring in websockets here too, so we know the exact channels the app is using to
/// communicate with the devserver. Currently that's a broadcast-type system, so this struct isn't super
/// duper useful.
pub(crate) struct AppHandle {
    pub(crate) app: AppBundle,

    // These might be None if the app died or the user did not specify a server
    pub(crate) app_child: Option<Child>,
    pub(crate) server_child: Option<Child>,

    // stdio for the app so we can read its stdout/stderr
    // we don't map stdin today (todo) but most apps don't need it
    pub(crate) app_stdout: Option<Lines<BufReader<ChildStdout>>>,
    pub(crate) app_stderr: Option<Lines<BufReader<ChildStderr>>>,
    pub(crate) server_stdout: Option<Lines<BufReader<ChildStdout>>>,
    pub(crate) server_stderr: Option<Lines<BufReader<ChildStderr>>>,

    /// The virtual directory that assets will be served from
    /// Used mostly for apk/ipa builds since they live in simulator
    pub(crate) runtime_asst_dir: Option<PathBuf>,
}

impl AppHandle {
    pub async fn new(app: AppBundle) -> Result<Self> {
        Ok(AppHandle {
            app,
            runtime_asst_dir: None,
            app_child: None,
            app_stderr: None,
            app_stdout: None,
            server_child: None,
            server_stdout: None,
            server_stderr: None,
        })
    }

    pub(crate) async fn open(
        &mut self,
        devserver_ip: SocketAddr,
        fullstack_address: Option<SocketAddr>,
        open_browser: bool,
    ) -> Result<()> {
        if let Some(addr) = fullstack_address {
            tracing::debug!("Proxying fullstack server from port {:?}", addr);
        }

        // Set the env vars that the clients will expect
        // These need to be stable within a release version (ie 0.6.0)
        let mut envs = vec![
            ("DIOXUS_CLI_ENABLED", "true".to_string()),
            (
                dioxus_cli_config::DEVSERVER_RAW_ADDR_ENV,
                devserver_ip.to_string(),
            ),
            // unset the cargo dirs in the event we're running `dx` locally
            // since the child process will inherit the env vars, we don't want to confuse the downstream process
            ("CARGO_MANIFEST_DIR", "".to_string()),
        ];

        if let Some(addr) = fullstack_address {
            envs.push((dioxus_cli_config::SERVER_IP_ENV, addr.ip().to_string()));
            envs.push((dioxus_cli_config::SERVER_PORT_ENV, addr.port().to_string()));
        }

        // Launch the server if we have one and consume its stdout/stderr
        if let Some(server) = self.app.server_exe() {
            tracing::debug!("Launching server from path: {server:?}");
            let mut child = Command::new(server)
                .envs(envs.clone())
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .kill_on_drop(true)
                .spawn()?;
            let stdout = BufReader::new(child.stdout.take().unwrap());
            let stderr = BufReader::new(child.stderr.take().unwrap());
            self.server_stdout = Some(stdout.lines());
            self.server_stderr = Some(stderr.lines());
            self.server_child = Some(child);
        }

        // We try to use stdin/stdout to communicate with the app
        let running_process = match self.app.build.build.platform() {
            // Unfortunately web won't let us get a proc handle to it (to read its stdout/stderr) so instead
            // use use the websocket to communicate with it. I wish we could merge the concepts here,
            // like say, opening the socket as a subprocess, but alas, it's simpler to do that somewhere else.
            Platform::Web => {
                // Only the first build we open the web app, after that the user knows it's running
                if open_browser {
                    self.open_web(envs, devserver_ip);
                }

                None
            }

            Platform::Ios => Some(self.open_ios_sim(envs).await?),

            // https://developer.android.com/studio/run/emulator-commandline
            Platform::Android => {
                tracing::error!("Android is not yet supported, sorry!");
                None
            }

            // These are all just basically running the main exe, but with slightly different resource dir paths
            Platform::Server
            | Platform::MacOS
            | Platform::Windows
            | Platform::Linux
            | Platform::Liveview => Some(self.open_with_main_exe(envs)?),
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

    /// Hotreload an asset in the running app.
    ///
    /// This will modify the build dir in place! Be careful! We generally assume you want all bundles
    /// to reflect the latest changes, so we will modify the bundle.
    ///
    /// However, not all platforms work like this, so we might also need to update a separate asset
    /// dir that the system simulator might be providing. We know this is the case for ios simulators
    /// and haven't yet checked for android.
    ///
    /// This will return the bundled name of the asset such that we can send it to the clients letting
    /// them know what to reload. It's not super important that this is robust since most clients will
    /// kick all stylsheets without necessarily checking the name.
    pub(crate) fn hotreload_bundled_asset(&self, changed_file: &PathBuf) -> Option<PathBuf> {
        let mut bundled_name = None;

        // Use the build dir if there's no runtime asset dir as the override. For the case of ios apps,
        // we won't actually be using the build dir.
        let asset_dir = match self.runtime_asst_dir.as_ref() {
            Some(dir) => dir.to_path_buf().join("assets/"),
            None => self.app.asset_dir(),
        };

        tracing::debug!("Hotreloading asset {changed_file:?} in target {asset_dir:?}");

        // If the asset shares the same name in the bundle, reload that
        let legacy_asset_dir = self.app.build.krate.legacy_asset_dir();
        if changed_file.starts_with(&legacy_asset_dir) {
            tracing::debug!("Hotreloading legacy asset {changed_file:?}");
            let trimmed = changed_file.strip_prefix(legacy_asset_dir).unwrap();
            let res = std::fs::copy(changed_file, asset_dir.join(trimmed));
            bundled_name = Some(trimmed.to_path_buf());
            if let Err(e) = res {
                tracing::debug!("Failed to hotreload legacy asset {e}");
            }
        }

        // The asset might've been renamed thanks to the manifest, let's attempt to reload that too
        if let Some(resource) = self.app.app.assets.assets.get(changed_file).as_ref() {
            let res = std::fs::copy(changed_file, asset_dir.join(&resource.bundled));
            bundled_name = Some(PathBuf::from(&resource.bundled));
            if let Err(e) = res {
                tracing::debug!("Failed to hotreload asset {e}");
            }
        }

        // Now we can return the bundled asset name to send to the hotreload engine
        bundled_name
    }

    /// Open the native app simply by running its main exe
    ///
    /// Eventually, for mac, we want to run the `.app` with `open` to fix issues with `dylib` paths,
    /// but for now, we just run the exe directly. Very few users should be caring about `dylib` search
    /// paths right now, but they will when we start to enable things like swift integration.
    ///
    /// Server/liveview/desktop are all basically the same, though
    fn open_with_main_exe(&mut self, envs: Vec<(&str, String)>) -> Result<Child> {
        let child = Command::new(self.app.main_exe())
            .envs(envs)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        Ok(child)
    }

    /// Open the web app by opening the browser to the given address.
    /// Check if we need to use https or not, and if so, add the protocol.
    /// Go to the basepath if that's set too.
    fn open_web(&self, _envs: Vec<(&str, String)>, address: SocketAddr) {
        let base_path = self.app.build.krate.config.web.app.base_path.clone();
        let https = self
            .app
            .build
            .krate
            .config
            .web
            .https
            .enabled
            .unwrap_or_default();
        let protocol = if https { "https" } else { "http" };
        let base_path = match base_path.as_deref() {
            Some(base_path) => format!("/{}", base_path.trim_matches('/')),
            None => "".to_owned(),
        };
        _ = open::that(format!("{protocol}://{address}{base_path}"));
    }

    /// Use `xcrun` to install the app to the simulator
    /// With simulators, we're free to basically do anything, so we don't need to do any fancy codesigning
    /// or entitlements, or anything like that.
    ///
    /// However, if there's no simulator running, this *might* fail.
    ///
    /// TODO(jon): we should probably check if there's a simulator running before trying to install,
    /// and open the simulator if we have to.
    async fn open_ios_sim(&mut self, envs: Vec<(&str, String)>) -> Result<Child> {
        tracing::debug!("Installing app to simulator {:?}", self.app.app_dir());

        let res = Command::new("xcrun")
            .arg("simctl")
            .arg("install")
            .arg("booted")
            .arg(self.app.app_dir())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .await?;

        tracing::debug!("Installed app to simulator with exit code: {res:?}");

        // Remap the envs to the correct simctl env vars
        // iOS sim lets you pass env vars but they need to be in the format "SIMCTL_CHILD_XXX=XXX"
        let ios_envs = envs
            .iter()
            .map(|(k, v)| (format!("SIMCTL_CHILD_{k}"), v.clone()));

        let child = Command::new("xcrun")
            .arg("simctl")
            .arg("launch")
            .arg("--console")
            .arg("booted")
            .arg("com.dioxuslabs")
            .envs(ios_envs)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        tracing::debug!("Launched app on simulator with exit code: {child:?}");

        Ok(child)
    }

    /// We have this whole thing figured out, but we don't actually use it yet.
    ///
    /// Launching on devices is more complicated and requires us to codesign the app, which we don't
    /// currently do.
    ///
    /// Converting these commands shouldn't be too hard, but device support would imply we need
    /// better support for codesigning and entitlements.
    #[allow(unused)]
    async fn open_ios_device(&self) -> Result<()> {
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

        use serde_json::Value;
        let app_path = self.app.app_dir();

        install_app(&app_path).await?;

        // 2. Determine which device the app was installed to
        let device_uuid = get_device_uuid().await?;

        // 3. Get the installation URL of the app
        let installation_url = get_installation_url(&device_uuid, &app_path).await?;

        // 4. Launch the app into the background, paused
        launch_app_paused(&device_uuid, &installation_url).await?;

        // 5. Pick up the paused app and resume it
        resume_app(&device_uuid).await?;

        async fn install_app(app_path: &PathBuf) -> Result<()> {
            let output = Command::new("xcrun")
                .args(["simctl", "install", "booted"])
                .arg(app_path)
                .output()
                .await?;

            if !output.status.success() {
                return Err(format!("Failed to install app: {:?}", output).into());
            }

            Ok(())
        }

        async fn get_device_uuid() -> Result<String> {
            let output = Command::new("xcrun")
                .args([
                    "devicectl",
                    "list",
                    "devices",
                    "--json-output",
                    "target/deviceid.json",
                ])
                .output()
                .await?;

            let json: Value =
                serde_json::from_str(&std::fs::read_to_string("target/deviceid.json")?)
                    .context("Failed to parse xcrun output")?;
            let device_uuid = json["result"]["devices"][0]["identifier"]
                .as_str()
                .ok_or("Failed to extract device UUID")?
                .to_string();

            Ok(device_uuid)
        }

        async fn get_installation_url(device_uuid: &str, app_path: &Path) -> Result<String> {
            let output = Command::new("xcrun")
                .args([
                    "devicectl",
                    "device",
                    "install",
                    "app",
                    "--device",
                    device_uuid,
                    &app_path.display().to_string(),
                    "--json-output",
                    "target/xcrun.json",
                ])
                .output()
                .await?;

            if !output.status.success() {
                return Err(format!("Failed to install app: {:?}", output).into());
            }

            let json: Value = serde_json::from_str(&std::fs::read_to_string("target/xcrun.json")?)
                .context("Failed to parse xcrun output")?;
            let installation_url = json["result"]["installedApplications"][0]["installationURL"]
                .as_str()
                .ok_or("Failed to extract installation URL")?
                .to_string();

            Ok(installation_url)
        }

        async fn launch_app_paused(device_uuid: &str, installation_url: &str) -> Result<()> {
            let output = Command::new("xcrun")
                .args([
                    "devicectl",
                    "device",
                    "process",
                    "launch",
                    "--no-activate",
                    "--verbose",
                    "--device",
                    device_uuid,
                    installation_url,
                    "--json-output",
                    "target/launch.json",
                ])
                .output()
                .await?;

            if !output.status.success() {
                return Err(format!("Failed to launch app: {:?}", output).into());
            }

            Ok(())
        }

        async fn resume_app(device_uuid: &str) -> Result<()> {
            let json: Value = serde_json::from_str(&std::fs::read_to_string("target/launch.json")?)
                .context("Failed to parse xcrun output")?;

            let status_pid = json["result"]["process"]["processIdentifier"]
                .as_u64()
                .ok_or("Failed to extract process identifier")?;

            let output = Command::new("xcrun")
                .args([
                    "devicectl",
                    "device",
                    "process",
                    "resume",
                    "--device",
                    device_uuid,
                    "--pid",
                    &status_pid.to_string(),
                ])
                .output()
                .await?;

            if !output.status.success() {
                return Err(format!("Failed to resume app: {:?}", output).into());
            }

            Ok(())
        }

        unimplemented!("dioxus-cli doesn't support ios devices yet.")
    }
}
