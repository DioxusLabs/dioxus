use crate::{
    BuildArgs, BuildArtifacts, BuildRequest, BuildStage, BuilderUpdate, Platform, ProgressRx,
    ProgressTx, Result, StructuredOutput,
};
use anyhow::Context;
use dioxus_cli_opt::process_file_to;
use futures_util::future::OptionFuture;
use std::time::{Duration, Instant};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
};
use subsecond_cli_support::JumpTable;
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{Child, ChildStderr, ChildStdout, Command},
    task::JoinHandle,
};

use super::{BuildContext, BuildMode};

/// The component of the serve engine that watches ongoing builds and manages their state, open handle,
/// and progress.
///
/// Previously, the builder allowed multiple apps to be built simultaneously, but this newer design
/// simplifies the code and allows only one app and its server to be built at a time.
///
/// Here, we track the number of crates being compiled, assets copied, the times of these events, and
/// other metadata that gives us useful indicators for the UI.
///
/// A handle to a running app.
///
/// The actual child processes might not be present (web) or running (died/killed).
///
/// The purpose of this struct is to accumulate state about the running app and its server, like
/// any runtime information needed to hotreload the app or send it messages.
///
/// We might want to bring in websockets here too, so we know the exact channels the app is using to
/// communicate with the devserver. Currently that's a broadcast-type system, so this struct isn't super
/// duper useful.
///
/// todo: restructure this such that "open" is a running task instead of blocking the main thread
pub(crate) struct AppBuilder {
    pub tx: ProgressTx,
    pub rx: ProgressRx,

    // The original request with access to its build directory
    pub build: BuildRequest,

    // Ongoing build task, if any
    pub build_task: JoinHandle<Result<BuildArtifacts>>,

    // If a build has already finished, we'll have its artifacts (rustc, link args, etc) to work with
    pub artifacts: Option<BuildArtifacts>,

    /// The aslr offset of this running app
    pub aslr_reference: Option<u64>,

    /// The list of patches applied to the app, used to know which ones to reapply and/or iterate from.
    pub patches: Vec<JumpTable>,

    /// The virtual directory that assets will be served from
    /// Used mostly for apk/ipa builds since they live in simulator
    pub runtime_asset_dir: Option<PathBuf>,

    // These might be None if the app died or the user did not specify a server
    pub child: Option<Child>,

    // stdio for the app so we can read its stdout/stderr
    // we don't map stdin today (todo) but most apps don't need it
    pub stdout: Option<Lines<BufReader<ChildStdout>>>,
    pub stderr: Option<Lines<BufReader<ChildStderr>>>,

    /// The executables but with some extra entropy in their name so we can run two instances of the
    /// same app without causing collisions on the filesystem.
    pub entropy_app_exe: Option<PathBuf>,

    // Metadata about the build that needs to be managed by watching build updates
    // used to render the TUI
    pub stage: BuildStage,
    pub compiled_crates: usize,
    pub expected_crates: usize,
    pub bundling_progress: f64,
    pub compile_start: Option<Instant>,
    pub compile_end: Option<Instant>,
    pub bundle_start: Option<Instant>,
    pub bundle_end: Option<Instant>,
}

impl AppBuilder {
    /// Create a new builder and immediately start a build
    pub(crate) fn start(request: &BuildRequest) -> Result<Self> {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        Ok(Self {
            build: request.clone(),
            stage: BuildStage::Initializing,
            build_task: tokio::spawn({
                let request = request.clone();
                let tx = tx.clone();
                async move {
                    let ctx = BuildContext {
                        tx: tx.clone(),
                        mode: BuildMode::Fat,
                    };
                    request.verify_tooling(&ctx).await?;
                    request.prepare_build_dir()?;
                    request.build(&ctx).await
                }
            }),
            tx,
            rx,
            patches: vec![],
            compiled_crates: 0,
            expected_crates: 1,
            bundling_progress: 0.0,
            compile_start: Some(Instant::now()),
            aslr_reference: None,
            compile_end: None,
            bundle_start: None,
            bundle_end: None,
            runtime_asset_dir: None,
            child: None,
            stderr: None,
            stdout: None,
            entropy_app_exe: None,
            artifacts: None,
        })
    }

    /// Wait for any new updates to the builder - either it completed or gave us a message etc
    pub(crate) async fn wait(&mut self) -> BuilderUpdate {
        use futures_util::StreamExt;
        use BuilderUpdate::*;

        // Wait for the build to finish or for it to emit a status message
        let update = tokio::select! {
            Some(progress) = self.rx.next() => progress,
            bundle = (&mut self.build_task) => {
                // Replace the build with an infinitely pending task so we can select it again without worrying about deadlocks/spins
                self.build_task = tokio::task::spawn(std::future::pending());
                match bundle {
                    Ok(Ok(bundle)) => BuilderUpdate::BuildReady { bundle },
                    Ok(Err(err)) => BuilderUpdate::BuildFailed { err },
                    Err(err) => BuilderUpdate::BuildFailed { err: crate::Error::Runtime(format!("Build panicked! {:?}", err)) },
                }
            },
            Some(Ok(Some(msg))) = OptionFuture::from(self.stdout.as_mut().map(|f| f.next_line())) => {
                StdoutReceived {  msg }
            },
            Some(Ok(Some(msg))) = OptionFuture::from(self.stderr.as_mut().map(|f| f.next_line())) => {
                StderrReceived {  msg }
            },
            Some(status) = OptionFuture::from(self.child.as_mut().map(|f| f.wait())) => {
                match status {
                    Ok(status) => {
                        self.child = None;
                        ProcessExited { status }
                    },
                    Err(_err) => todo!("handle error in process joining?"),
                }
            }
        };

        // Update the internal stage of the build so the UI can render it
        // *VERY IMPORTANT* - DO NOT AWAIT HERE
        // doing so will cause the changes to be lost since this wait call is called under a cancellable task
        // todo - move this handling to a separate function that won't be cancelled
        match &update {
            BuilderUpdate::Progress { stage } => {
                // Prevent updates from flowing in after the build has already finished
                if !self.is_finished() {
                    self.stage = stage.clone();

                    match stage {
                        BuildStage::Initializing => {
                            self.compiled_crates = 0;
                            self.bundling_progress = 0.0;
                        }
                        BuildStage::Starting { crate_count, .. } => {
                            self.expected_crates = *crate_count;
                        }
                        BuildStage::InstallingTooling {} => {}
                        BuildStage::Compiling { current, total, .. } => {
                            self.compiled_crates = *current;
                            self.expected_crates = *total;

                            if self.compile_start.is_none() {
                                self.compile_start = Some(Instant::now());
                            }
                        }
                        BuildStage::Bundling {} => {
                            self.complete_compile();
                            self.bundling_progress = 0.0;
                            self.bundle_start = Some(Instant::now());
                        }
                        BuildStage::OptimizingWasm {} => {}
                        BuildStage::CopyingAssets { current, total, .. } => {
                            self.bundling_progress = *current as f64 / *total as f64;
                        }
                        BuildStage::Success => {
                            self.compiled_crates = self.expected_crates;
                            self.bundling_progress = 1.0;
                        }
                        BuildStage::Failed => {
                            self.compiled_crates = self.expected_crates;
                            self.bundling_progress = 1.0;
                        }
                        BuildStage::Aborted => {}
                        BuildStage::Restarting => {
                            self.compiled_crates = 0;
                            self.expected_crates = 1;
                            self.bundling_progress = 0.0;
                        }
                        BuildStage::RunningBindgen {} => {}
                        _ => {}
                    }
                }
            }
            BuilderUpdate::CompilerMessage { .. } => {}
            BuilderUpdate::BuildReady { .. } => {
                self.compiled_crates = self.expected_crates;
                self.bundling_progress = 1.0;
                self.stage = BuildStage::Success;

                self.complete_compile();
                self.bundle_end = Some(Instant::now());
            }
            BuilderUpdate::BuildFailed { .. } => {
                tracing::debug!("Setting builder to failed state");
                self.stage = BuildStage::Failed;
            }
            StdoutReceived { .. } => {}
            StderrReceived { .. } => {}
            ProcessExited { .. } => {}
        }

        update
    }

    pub(crate) fn patch_rebuild(&mut self, changed_files: Vec<PathBuf>) {
        // Abort all the ongoing builds, cleaning up any loose artifacts and waiting to cleanly exit
        self.abort_all();
        self.stage = BuildStage::Restarting;

        // This build doesn't have any extra special logging - rebuilds would get pretty noisy
        let request = self.build.clone();
        let ctx = BuildContext {
            tx: self.tx.clone(),
            mode: BuildMode::Thin {
                changed_files,
                direct_rustc: self.artifacts.as_ref().unwrap().direct_rustc.clone(),
                aslr_reference: self.aslr_reference.unwrap(),
            },
        };
        self.build_task = tokio::spawn(async move { request.build(&ctx).await });
    }

    /// Restart this builder with new build arguments.
    pub(crate) fn rebuild(&mut self) {
        // Abort all the ongoing builds, cleaning up any loose artifacts and waiting to cleanly exit
        // And then start a new build, resetting our progress/stage to the beginning and replacing the old tokio task
        self.abort_all();
        self.stage = BuildStage::Restarting;

        // This build doesn't have any extra special logging - rebuilds would get pretty noisy
        let request = self.build.clone();
        let ctx = BuildContext {
            tx: self.tx.clone(),
            mode: BuildMode::Fat,
        };
        self.build_task = tokio::spawn(async move { request.build(&ctx).await });
    }

    /// Shutdown the current build process
    ///
    /// todo: might want to use a cancellation token here to allow cleaner shutdowns
    pub(crate) fn abort_all(&mut self) {
        self.build_task.abort();
        self.stage = BuildStage::Aborted;
        self.compiled_crates = 0;
        self.expected_crates = 1;
        self.bundling_progress = 0.0;
        self.compile_start = None;
        self.bundle_start = None;
        self.bundle_end = None;
        self.compile_end = None;
    }

    /// Wait for the build to finish, returning the final bundle
    /// Should only be used by code that's not interested in the intermediate updates and only cares about the final bundle
    ///
    /// todo(jon): maybe we want to do some logging here? The build/bundle/run screens could be made to
    /// use the TUI output for prettier outputs.
    pub(crate) async fn finish_build(&mut self) -> Result<BuildArtifacts> {
        loop {
            match self.wait().await {
                BuilderUpdate::Progress { stage } => {
                    match &stage {
                        BuildStage::Compiling {
                            current,
                            total,
                            krate,
                            ..
                        } => {
                            tracing::info!("Compiling [{current:>3}/{total}]: {krate}");
                        }
                        BuildStage::RunningBindgen => tracing::info!("Running wasm-bindgen..."),
                        BuildStage::CopyingAssets {
                            current,
                            total,
                            path,
                        } => {
                            tracing::info!("Copying asset ({current}/{total}): {}", path.display());
                        }
                        BuildStage::Bundling => tracing::info!("Bundling app..."),
                        _ => {}
                    }

                    tracing::info!(json = ?StructuredOutput::BuildUpdate { stage: stage.clone() });
                }
                BuilderUpdate::CompilerMessage { message } => {
                    tracing::info!(json = ?StructuredOutput::CargoOutput { message: message.clone() }, %message);
                }
                BuilderUpdate::BuildReady { bundle } => {
                    tracing::debug!(json = ?StructuredOutput::BuildFinished {
                        path: self.build.root_dir(),
                    });
                    return Ok(bundle);
                }
                BuilderUpdate::BuildFailed { err } => {
                    // Flush remaining compiler messages
                    while let Ok(Some(msg)) = self.rx.try_next() {
                        if let BuilderUpdate::CompilerMessage { message } = msg {
                            tracing::info!(json = ?StructuredOutput::CargoOutput { message: message.clone() }, %message);
                        }
                    }

                    tracing::error!(?err, json = ?StructuredOutput::Error { message: err.to_string() });
                    return Err(err);
                }
                BuilderUpdate::StdoutReceived { msg } => {}
                BuilderUpdate::StderrReceived { msg } => {}
                BuilderUpdate::ProcessExited { status } => {}
            }
        }
    }

    pub(crate) async fn open(
        &mut self,
        devserver_ip: SocketAddr,
        start_fullstack_on_address: Option<SocketAddr>,
        open_browser: bool,
        always_on_top: bool,
    ) -> Result<()> {
        let krate = &self.build;

        // Set the env vars that the clients will expect
        // These need to be stable within a release version (ie 0.6.0)
        let mut envs = vec![
            (dioxus_cli_config::CLI_ENABLED_ENV, "true".to_string()),
            (
                dioxus_cli_config::ALWAYS_ON_TOP_ENV,
                always_on_top.to_string(),
            ),
            (
                dioxus_cli_config::APP_TITLE_ENV,
                krate.config.web.app.title.clone(),
            ),
            ("RUST_BACKTRACE", "1".to_string()),
            (
                dioxus_cli_config::DEVSERVER_IP_ENV,
                devserver_ip.ip().to_string(),
            ),
            (
                dioxus_cli_config::DEVSERVER_PORT_ENV,
                devserver_ip.port().to_string(),
            ),
            // unset the cargo dirs in the event we're running `dx` locally
            // since the child process will inherit the env vars, we don't want to confuse the downstream process
            ("CARGO_MANIFEST_DIR", "".to_string()),
            (
                dioxus_cli_config::SESSION_CACHE_DIR,
                self.build.session_cache_dir().display().to_string(),
            ),
        ];

        if let Some(base_path) = &krate.config.web.app.base_path {
            envs.push((dioxus_cli_config::ASSET_ROOT_ENV, base_path.clone()));
        }

        // Launch the server if we were given an address to start it on, and the build includes a server. After we
        // start the server, consume its stdout/stderr.
        if let Some(addr) = start_fullstack_on_address {
            envs.push((dioxus_cli_config::SERVER_IP_ENV, addr.ip().to_string()));
            envs.push((dioxus_cli_config::SERVER_PORT_ENV, addr.port().to_string()));
        }

        // We try to use stdin/stdout to communicate with the app
        let running_process = match self.build.platform {
            // Unfortunately web won't let us get a proc handle to it (to read its stdout/stderr) so instead
            // use use the websocket to communicate with it. I wish we could merge the concepts here,
            // like say, opening the socket as a subprocess, but alas, it's simpler to do that somewhere else.
            Platform::Web => {
                // Only the first build we open the web app, after that the user knows it's running
                if open_browser {
                    self.open_web(devserver_ip);
                }

                None
            }

            Platform::Ios => Some(self.open_ios_sim(envs).await?),

            // https://developer.android.com/studio/run/emulator-commandline
            Platform::Android => {
                self.open_android_sim(devserver_ip, envs).await;
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
            self.stdout = Some(stdout.lines());
            self.stderr = Some(stderr.lines());
            self.child = Some(child);
        }

        Ok(())
    }

    /// Gracefully kill the process and all of its children
    ///
    /// Uses the `SIGTERM` signal on unix and `taskkill` on windows.
    /// This complex logic is necessary for things like window state preservation to work properly.
    ///
    /// Also wipes away the entropy executables if they exist.
    pub(crate) async fn cleanup(&mut self) {
        // Soft-kill the process by sending a sigkill, allowing the process to clean up
        self.soft_kill().await;

        // Wipe out the entropy executables if they exist
        if let Some(entropy_app_exe) = self.entropy_app_exe.take() {
            _ = std::fs::remove_file(entropy_app_exe);
        }

        // if matches!(self.platform, Platform::Android) {
        //     use std::process::{Command, Stdio};
        //     if let Err(err) = Command::new("adb")
        //         .arg("reverse")
        //         .arg("--remove")
        //         .arg(format!("tcp:{}", self.devserver_port))
        //         .stderr(Stdio::piped())
        //         .stdout(Stdio::piped())
        //         .output()
        //     {
        //         tracing::error!(
        //             "failed to remove forwarded port {}: {err}",
        //             self.devserver_port
        //         );
        //     }
        // }
    }

    /// Kill the app and server exes
    pub(crate) async fn soft_kill(&mut self) {
        use futures_util::FutureExt;

        // Kill any running executables on Windows
        let Some(mut process) = self.child.take() else {
            return;
        };

        let Some(pid) = process.id() else {
            _ = process.kill().await;
            return;
        };

        // on unix, we can send a signal to the process to shut down
        #[cfg(unix)]
        {
            _ = Command::new("kill")
                .args(["-s", "TERM", &pid.to_string()])
                .spawn();
        }

        // on windows, use the `taskkill` command
        #[cfg(windows)]
        {
            _ = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .spawn();
        }

        // join the wait with a 100ms timeout
        futures_util::select! {
            _ = process.wait().fuse() => {}
            _ = tokio::time::sleep(std::time::Duration::from_millis(1000)).fuse() => {}
        };
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
    pub(crate) async fn hotreload_bundled_asset(&self, changed_file: &PathBuf) -> Option<PathBuf> {
        let mut bundled_name = None;

        let Some(artifacts) = self.artifacts.as_ref() else {
            tracing::debug!("No artifacts to hotreload asset");
            return None;
        };

        // Use the build dir if there's no runtime asset dir as the override. For the case of ios apps,
        // we won't actually be using the build dir.
        let asset_dir = match self.runtime_asset_dir.as_ref() {
            Some(dir) => dir.to_path_buf().join("assets/"),
            None => self.build.asset_dir(),
        };

        tracing::debug!("Hotreloading asset {changed_file:?} in target {asset_dir:?}");

        // If the asset shares the same name in the bundle, reload that
        if let Some(legacy_asset_dir) = self.build.legacy_asset_dir() {
            if changed_file.starts_with(&legacy_asset_dir) {
                tracing::debug!("Hotreloading legacy asset {changed_file:?}");
                let trimmed = changed_file.strip_prefix(legacy_asset_dir).unwrap();
                let res = std::fs::copy(changed_file, asset_dir.join(trimmed));
                bundled_name = Some(trimmed.to_path_buf());
                if let Err(e) = res {
                    tracing::debug!("Failed to hotreload legacy asset {e}");
                }
            }
        }

        // Canonicalize the path as Windows may use long-form paths "\\\\?\\C:\\".
        let changed_file = dunce::canonicalize(changed_file)
            .inspect_err(|e| tracing::debug!("Failed to canonicalize hotreloaded asset: {e}"))
            .ok()?;

        // The asset might've been renamed thanks to the manifest, let's attempt to reload that too
        if let Some(resource) = artifacts.assets.assets.get(&changed_file).as_ref() {
            let output_path = asset_dir.join(resource.bundled_path());
            // Remove the old asset if it exists
            _ = std::fs::remove_file(&output_path);
            // And then process the asset with the options into the **old** asset location. If we recompiled,
            // the asset would be in a new location because the contents and hash have changed. Since we are
            // hotreloading, we need to use the old asset location it was originally written to.
            let options = *resource.options();
            let res = process_file_to(&options, &changed_file, &output_path);
            bundled_name = Some(PathBuf::from(resource.bundled_path()));
            if let Err(e) = res {
                tracing::debug!("Failed to hotreload asset {e}");
            }
        }

        // If the emulator is android, we need to copy the asset to the device with `adb push asset /data/local/tmp/dx/assets/filename.ext`
        if self.build.platform == Platform::Android {
            if let Some(bundled_name) = bundled_name.as_ref() {
                _ = self
                    .copy_file_to_android_tmp(&changed_file, &bundled_name)
                    .await;
            }
        }

        // Now we can return the bundled asset name to send to the hotreload engine
        bundled_name
    }

    /// Copy this file to the tmp folder on the android device, returning the path to the copied file
    pub(crate) async fn copy_file_to_android_tmp(
        &self,
        changed_file: &Path,
        bundled_name: &Path,
    ) -> Result<PathBuf> {
        let target = PathBuf::from("/data/app/~~OE9KIaCNz0l5pwJue6zY8Q==/com.example.SubsecondHarness-pilWFhddpEHdzmzy-khHRA==/lib/arm64/").join(bundled_name);
        // let target = dioxus_cli_config::android_session_cache_dir().join(bundled_name);
        tracing::debug!("Pushing asset to device: {target:?}");
        let res = tokio::process::Command::new(crate::build::android_tools().unwrap().adb)
            .arg("push")
            .arg(&changed_file)
            .arg(&target)
            .output()
            .await
            .context("Failed to push asset to device");

        if let Err(e) = res {
            tracing::debug!("Failed to push asset to device: {e}");
        }

        Ok(target)
    }

    /// Open the native app simply by running its main exe
    ///
    /// Eventually, for mac, we want to run the `.app` with `open` to fix issues with `dylib` paths,
    /// but for now, we just run the exe directly. Very few users should be caring about `dylib` search
    /// paths right now, but they will when we start to enable things like swift integration.
    ///
    /// Server/liveview/desktop are all basically the same, though
    fn open_with_main_exe(&mut self, envs: Vec<(&str, String)>) -> Result<Child> {
        // Create a new entropy app exe if we need to
        let main_exe = self.app_exe();
        let child = Command::new(main_exe)
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
    fn open_web(&self, address: SocketAddr) {
        let base_path = self.build.config.web.app.base_path.clone();
        let https = self.build.config.web.https.enabled.unwrap_or_default();
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
        tracing::debug!("Installing app to simulator {:?}", self.build.root_dir());

        let res = Command::new("xcrun")
            .arg("simctl")
            .arg("install")
            .arg("booted")
            .arg(self.build.root_dir())
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
            .arg(self.build.bundle_identifier())
            .envs(ios_envs)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

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
        use serde_json::Value;
        let app_path = self.build.root_dir();

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
            // xcrun devicectl device install app --device <uuid> --path <path> --json-output
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

    #[allow(unused)]
    async fn codesign_ios(&self) -> Result<()> {
        const CODESIGN_ERROR: &str = r#"This is likely because you haven't
- Created a provisioning profile before
- Accepted the Apple Developer Program License Agreement

The agreement changes frequently and might need to be accepted again.
To accept the agreement, go to https://developer.apple.com/account

To create a provisioning profile, follow the instructions here:
https://developer.apple.com/documentation/xcode/sharing-your-teams-signing-certificates"#;

        let profiles_folder = dirs::home_dir()
            .context("Your machine has no home-dir")?
            .join("Library/MobileDevice/Provisioning Profiles");

        if !profiles_folder.exists() || profiles_folder.read_dir()?.next().is_none() {
            tracing::error!(
                r#"No provisioning profiles found when trying to codesign the app.
We checked the folder: {}

{CODESIGN_ERROR}
"#,
                profiles_folder.display()
            )
        }

        let identities = Command::new("security")
            .args(["find-identity", "-v", "-p", "codesigning"])
            .output()
            .await
            .context("Failed to run `security find-identity -v -p codesigning`")
            .map(|e| {
                String::from_utf8(e.stdout)
                    .context("Failed to parse `security find-identity -v -p codesigning`")
            })??;

        // Parsing this:
        // 51ADE4986E0033A5DB1C794E0D1473D74FD6F871 "Apple Development: jkelleyrtp@gmail.com (XYZYZY)"
        let app_dev_name = regex::Regex::new(r#""Apple Development: (.+)""#)
            .unwrap()
            .captures(&identities)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
            .context(
                "Failed to find Apple Development in `security find-identity -v -p codesigning`",
            )?;

        // Acquire the provision file
        let provision_file = profiles_folder
            .read_dir()?
            .flatten()
            .find(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|s| s.contains("mobileprovision"))
                    .unwrap_or_default()
            })
            .context("Failed to find a provisioning profile. \n\n{CODESIGN_ERROR}")?;

        // The .mobileprovision file has some random binary thrown into into, but it's still basically a plist
        // Let's use the plist markers to find the start and end of the plist
        fn cut_plist(bytes: &[u8], byte_match: &[u8]) -> Option<usize> {
            bytes
                .windows(byte_match.len())
                .enumerate()
                .rev()
                .find(|(_, slice)| *slice == byte_match)
                .map(|(i, _)| i + byte_match.len())
        }
        let bytes = std::fs::read(provision_file.path())?;
        let cut1 = cut_plist(&bytes, b"<plist").context("Failed to parse .mobileprovision file")?;
        let cut2 = cut_plist(&bytes, r#"</dict>"#.as_bytes())
            .context("Failed to parse .mobileprovision file")?;
        let sub_bytes = &bytes[(cut1 - 6)..cut2];
        let mbfile: ProvisioningProfile =
            plist::from_bytes(sub_bytes).context("Failed to parse .mobileprovision file")?;

        #[derive(serde::Deserialize, Debug)]
        struct ProvisioningProfile {
            #[serde(rename = "TeamIdentifier")]
            team_identifier: Vec<String>,
            #[serde(rename = "ApplicationIdentifierPrefix")]
            application_identifier_prefix: Vec<String>,
            #[serde(rename = "Entitlements")]
            entitlements: Entitlements,
        }

        #[derive(serde::Deserialize, Debug)]
        struct Entitlements {
            #[serde(rename = "application-identifier")]
            application_identifier: String,
            #[serde(rename = "keychain-access-groups")]
            keychain_access_groups: Vec<String>,
        }

        let entielements_xml = format!(
            r#"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>application-identifier</key>
    <string>{APPLICATION_IDENTIFIER}</string>
    <key>keychain-access-groups</key>
    <array>
        <string>{APP_ID_ACCESS_GROUP}.*</string>
    </array>
    <key>get-task-allow</key>
    <true/>
    <key>com.apple.developer.team-identifier</key>
    <string>{TEAM_IDENTIFIER}</string>
</dict></plist>
        "#,
            APPLICATION_IDENTIFIER = mbfile.entitlements.application_identifier,
            APP_ID_ACCESS_GROUP = mbfile.entitlements.keychain_access_groups[0],
            TEAM_IDENTIFIER = mbfile.team_identifier[0],
        );

        // write to a temp file
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(temp_file.path(), entielements_xml)?;

        // codesign the app
        let output = Command::new("codesign")
            .args([
                "--force",
                "--entitlements",
                temp_file.path().to_str().unwrap(),
                "--sign",
                app_dev_name,
            ])
            .arg(self.build.root_dir())
            .output()
            .await
            .context("Failed to codesign the app")?;

        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr).unwrap_or_default();
            return Err(format!("Failed to codesign the app: {stderr}").into());
        }

        Ok(())
    }

    async fn open_android_sim(
        &self,
        devserver_socket: SocketAddr,
        envs: Vec<(&'static str, String)>,
    ) {
        let apk_path = self.build.apk_path();
        let session_cache = self.build.session_cache_dir();
        let full_mobile_app_name = self.build.full_mobile_app_name();

        // Start backgrounded since .open() is called while in the arm of the top-level match
        tokio::task::spawn(async move {
            let adb = crate::build::android_tools().unwrap().adb;

            let port = devserver_socket.port();
            if let Err(e) = Command::new("adb")
                .arg("reverse")
                .arg(format!("tcp:{}", port))
                .arg(format!("tcp:{}", port))
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .output()
                .await
            {
                tracing::error!("failed to forward port {port}: {e}");
            }

            // Install
            // adb install -r app-debug.apk
            if let Err(e) = Command::new(&adb)
                .arg("install")
                .arg("-r")
                .arg(apk_path)
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .output()
                .await
            {
                tracing::error!("Failed to install apk with `adb`: {e}");
            };

            // Write the env vars to a .env file in our session cache
            let env_file = session_cache.join(".env");
            let contents: String = envs
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join("\n");
            _ = std::fs::write(&env_file, contents);

            // Push the env file to the device
            if let Err(e) = tokio::process::Command::new(&adb)
                .arg("push")
                .arg(env_file)
                .arg(dioxus_cli_config::android_session_cache_dir().join(".env"))
                .output()
                .await
                .context("Failed to push asset to device")
            {
                tracing::error!("Failed to push .env file to device: {e}");
            }

            // eventually, use the user's MainActivity, not our MainActivity
            // adb shell am start -n dev.dioxus.main/dev.dioxus.main.MainActivity
            let activity_name = format!("{}/dev.dioxus.main.MainActivity", full_mobile_app_name,);

            if let Err(e) = Command::new(&adb)
                .arg("shell")
                .arg("am")
                .arg("start")
                .arg("-n")
                .arg(activity_name)
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .output()
                .await
            {
                tracing::error!("Failed to start app with `adb`: {e}");
            };
        });
    }

    fn make_entropy_path(exe: &PathBuf) -> PathBuf {
        let id = uuid::Uuid::new_v4();
        let name = id.to_string();
        let some_entropy = name.split('-').next().unwrap();

        // Make a copy of the server exe with a new name
        let entropy_server_exe = exe.with_file_name(format!(
            "{}-{}",
            exe.file_name().unwrap().to_str().unwrap(),
            some_entropy
        ));

        std::fs::copy(exe, &entropy_server_exe).unwrap();

        entropy_server_exe
    }

    fn server_exe(&mut self) -> Option<PathBuf> {
        todo!()
        // let mut server = self.app.server_exe()?;

        // // Create a new entropy server exe if we need to
        // if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
        //     // If we already have an entropy server exe, return it - this is useful for re-opening the same app
        //     if let Some(existing_server) = self.entropy_server_exe.clone() {
        //         return Some(existing_server);
        //     }

        //     // Otherwise, create a new entropy server exe and save it for re-opning
        //     let entropy_server_exe = Self::make_entropy_path(&server);
        //     self.entropy_server_exe = Some(entropy_server_exe.clone());
        //     server = entropy_server_exe;
        // }

        // Some(server)
    }

    fn app_exe(&mut self) -> PathBuf {
        let mut main_exe = self.build.main_exe();

        // The requirement here is based on the platform, not necessarily our current architecture.
        let requires_entropy = match self.build.platform {
            // When running "bundled", we don't need entropy
            Platform::Web => false,
            Platform::MacOS => false,
            Platform::Ios => false,
            Platform::Android => false,

            // But on platforms that aren't running as "bundled", we do.
            Platform::Windows => true,
            Platform::Linux => true,
            Platform::Server => true,
            Platform::Liveview => true,
        };

        if requires_entropy || std::env::var("DIOXUS_ENTROPY").is_ok() {
            // If we already have an entropy app exe, return it - this is useful for re-opening the same app
            if let Some(existing_app_exe) = self.entropy_app_exe.clone() {
                return existing_app_exe;
            }

            let entropy_app_exe = Self::make_entropy_path(&main_exe);
            self.entropy_app_exe = Some(entropy_app_exe.clone());
            main_exe = entropy_app_exe;
        }

        main_exe
    }

    fn complete_compile(&mut self) {
        if self.compile_end.is_none() {
            self.compiled_crates = self.expected_crates;
            self.compile_end = Some(Instant::now());
            // self.compile_end_server = Some(Instant::now());
        }
    }

    /// Get the total duration of the build, if all stages have completed
    pub(crate) fn total_build_time(&self) -> Option<Duration> {
        Some(self.compile_duration()? + self.bundle_duration()?)
    }

    pub(crate) fn compile_duration(&self) -> Option<Duration> {
        Some(
            self.compile_end
                .unwrap_or_else(Instant::now)
                .duration_since(self.compile_start?),
        )
    }

    pub(crate) fn bundle_duration(&self) -> Option<Duration> {
        Some(
            self.bundle_end
                .unwrap_or_else(Instant::now)
                .duration_since(self.bundle_start?),
        )
    }

    /// Return a number between 0 and 1 representing the progress of the app build
    pub(crate) fn compile_progress(&self) -> f64 {
        self.compiled_crates as f64 / self.expected_crates as f64
    }

    pub(crate) fn bundle_progress(&self) -> f64 {
        self.bundling_progress
    }

    pub(crate) fn is_finished(&self) -> bool {
        match self.stage {
            BuildStage::Success => true,
            BuildStage::Failed => true,
            BuildStage::Aborted => true,
            BuildStage::Restarting => false,
            _ => false,
        }
    }

    /// Check if the queued build is blocking hotreloads
    pub(crate) fn can_receive_hotreloads(&self) -> bool {
        matches!(&self.stage, BuildStage::Success)
    }
}
