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
    pub(crate) app: AppBundle,

    // Output where we put the final bundle
    pub(crate) bundle_dir: PathBuf,

    pub(crate) out_file: PathBuf,
    pub(crate) server: Option<PathBuf>,

    pub(crate) app_child: Option<Child>,
    pub(crate) server_child: Option<Child>,

    pub(crate) app_stdout: Option<Lines<BufReader<ChildStdout>>>,
    pub(crate) app_stderr: Option<Lines<BufReader<ChildStderr>>>,
    pub(crate) server_stdout: Option<Lines<BufReader<ChildStdout>>>,
    pub(crate) server_stderr: Option<Lines<BufReader<ChildStderr>>>,
}

impl AppHandle {
    pub fn start(
        app: AppBundle,
        devserver_ip: SocketAddr,
        fullstack_address: Option<SocketAddr>,
    ) -> Result<Self> {
        let platform = app.build.build.platform();
        let ip = devserver_ip.to_string();

        if platform == Platform::Server || app.build.build.fullstack {
            tracing::debug!(
                "Proxying fullstack server from port {:?}",
                fullstack_address
            );
        }

        let bundle_dir = app.build.krate.bundle_dir(platform);
        std::fs::create_dir_all(&bundle_dir)?;

        let out_file = app.finish(bundle_dir.clone())?;
        let server = app.copy_server(&bundle_dir)?;

        let mut handle = AppHandle {
            _id: Uuid::new_v4(),
            app,
            out_file,
            server,
            bundle_dir: bundle_dir.clone(),
            app_child: None,
            app_stderr: None,
            app_stdout: None,
            server_child: None,
            server_stdout: None,
            server_stderr: None,
        };

        // Set the env vars that the clients will expect
        // These need to be stable within a release version (ie 0.6.0)
        let mut envs = vec![
            ("DIOXUS_CLI_ENABLED", "true".to_string()),
            (
                "CARGO_MANIFEST_DIR",
                handle.app.build.krate.crate_dir().display().to_string(),
            ),
            (
                "SIMCTL_CHILD_CARGO_MANIFEST_DIR",
                handle.app.build.krate.crate_dir().display().to_string(),
            ),
            (
                dioxus_cli_config::DEVSERVER_RAW_ADDR_ENV,
                devserver_ip.to_string(),
            ),
        ];

        if let Some(addr) = fullstack_address {
            envs.push((dioxus_cli_config::SERVER_IP_ENV, addr.ip().to_string()));
            envs.push((dioxus_cli_config::SERVER_PORT_ENV, addr.port().to_string()));
        };

        // (
        //     dioxus_cli_config::ALWAYS_ON_TOP_ENV,
        //     serve.always_on_top.unwrap_or(true).to_string(),
        // ),
        // cmd.env(
        //     dioxus_cli_config::ASSET_ROOT_ENV,
        //     asset_root.display().to_string(),
        // );
        // cmd.env(
        //     dioxus_cli_config::DEVSERVER_RAW_ADDR_ENV,
        //     devserver_addr.to_string(),
        // );
        // cmd.env(dioxus_cli_config::APP_TITLE_ENV, app_title);
        // cmd.env(dioxus_cli_config::OUT_DIR, out_dir.display().to_string());

        // Launch the server if we have one
        if let Some(server) = handle.server.clone() {
            let mut cmd = Command::new(server);
            cmd.envs(envs.clone());
            cmd.current_dir(bundle_dir);
            cmd.stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .kill_on_drop(true);
            let mut child = cmd.spawn()?;
            let stdout = BufReader::new(child.stdout.take().unwrap());
            let stderr = BufReader::new(child.stderr.take().unwrap());
            handle.server_stdout = Some(stdout.lines());
            handle.server_stderr = Some(stderr.lines());
            handle.server_child = Some(child);
        }

        match platform {
            Platform::Web => {
                // tracing::info!(dx_src = ?TraceSrc::Dev, "Serving web app on http://{} 🎉", ip);
            }
            Platform::Desktop => {
                // tracing::info!(dx_src = ?TraceSrc::Dev, "Launching desktop app 🎉");
                tracing::debug!(dx_src = ?TraceSrc::Dev, "Desktop app location: {:?}", handle.out_file.display());
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

        // open the exe with some arguments/envvars/etc
        // we're going to try and configure this binary from the environment, if we can
        //
        // web can't be configured like this, so instead, we'll need to plumb a meta tag into the
        // index.html during dev
        match handle.app.build.build.platform() {
            Platform::Desktop => {
                let mut cmd = Command::new(handle.out_file.clone());
                cmd.envs(envs)
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped())
                    .kill_on_drop(true);

                let mut child = cmd.spawn()?;
                let stdout = BufReader::new(child.stdout.take().unwrap());
                let stderr = BufReader::new(child.stderr.take().unwrap());
                handle.app_stdout = Some(stdout.lines());
                handle.app_stderr = Some(stderr.lines());
                handle.app_child = Some(child);
            }
            Platform::Liveview => panic!(),
            Platform::Server => panic!(),
            Platform::Web => {}
            Platform::Ios => {}
            Platform::Android => {}
        }

        Ok(handle)
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

    pub(crate) fn runtime_asset_dir(&self) -> PathBuf {
        let dir = match self.app.build.build.platform() {
            Platform::Web => self.out_file.join("assets"),
            Platform::Desktop => todo!(),
            Platform::Ios => todo!(),
            Platform::Android => todo!(),
            Platform::Server => todo!(),
            Platform::Liveview => todo!(),
        };

        tracing::debug!("Runtime asset dir: {dir:?}");

        dir
    }
}

// let start_browser = args.open.unwrap_or_default();
// let base_path = cfg.dioxus_config.web.app.base_path.clone();
// let platform = args.platform();
// // Open the browser
// if start_browser && platform != Platform::Desktop {
//     open_browser(base_path, addr, rustls.is_some());
// }

// /// Open the browser to the address
// pub(crate) fn open_browser(base_path: Option<String>, address: SocketAddr, https: bool) {
//     let protocol = if https { "https" } else { "http" };
//     let base_path = match base_path.as_deref() {
//         Some(base_path) => format!("/{}", base_path.trim_matches('/')),
//         None => "".to_owned(),
//     };
//     _ = open::that(format!("{protocol}://{address}{base_path}"));
// }
