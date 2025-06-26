use crate::{
    serve::WebServer, BuildArtifacts, BuildRequest, BuildStage, BuilderUpdate, Platform,
    ProgressRx, ProgressTx, Result, StructuredOutput,
};
use anyhow::Context;
use dioxus_cli_opt::process_file_to;
use futures_util::{future::OptionFuture, pin_mut, FutureExt};
use itertools::Itertools;
use std::{
    env,
    time::{Duration, Instant, SystemTime},
};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Stdio,
};
use subsecond_types::JumpTable;
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{Child, ChildStderr, ChildStdout, Command},
    task::JoinHandle,
};

use super::{BuildContext, BuildId, BuildMode, HotpatchModuleCache};

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
    pub patch_cache: Option<HotpatchModuleCache>,

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
    pub builds_opened: usize,

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

    /// The debugger for the app - must be enabled with the `d` key
    pub(crate) pid: Option<u32>,
}

impl AppBuilder {
    /// Create a new `AppBuilder` and immediately start a build process.
    ///
    /// This method initializes the builder with the provided `BuildRequest` and spawns an asynchronous
    /// task (`build_task`) to handle the build process. The build process involves several stages:
    ///
    /// 1. **Tooling Verification**: Ensures that the necessary tools are available for the build.
    /// 2. **Build Directory Preparation**: Sets up the directory structure required for the build.
    /// 3. **Build Execution**: Executes the build process asynchronously.
    /// 4. **Bundling**: Packages the built artifacts into a final bundle.
    ///
    /// The `build_task` is a Tokio task that runs the build process in the background. It uses a
    /// `BuildContext` to manage the build state and communicate progress or errors via a message
    /// channel (`tx`).
    ///
    /// The builder is initialized with default values for various fields, such as the build stage,
    /// progress metrics, and optional runtime configurations.
    ///
    /// # Notes
    ///
    /// - The `build_task` is immediately spawned and will run independently of the caller.
    /// - The caller can use other methods on the `AppBuilder` to monitor the build progress or handle
    ///   updates (e.g., `wait`, `finish_build`).
    /// - The build process is designed to be cancellable and restartable using methods like `abort_all`
    ///   or `rebuild`.
    pub(crate) fn start(request: &BuildRequest, mode: BuildMode) -> Result<Self> {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        Ok(Self {
            build: request.clone(),
            stage: BuildStage::Initializing,
            build_task: tokio::spawn({
                let request = request.clone();
                let tx = tx.clone();
                async move {
                    let ctx = BuildContext {
                        mode,
                        tx: tx.clone(),
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
            builds_opened: 0,
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
            patch_cache: None,
            pid: None,
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
                    Err(err) => BuilderUpdate::BuildFailed { err: crate::Error::Runtime(format!("Build panicked! {:#?}", err)) },
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
                    Err(err) => {
                        let () = futures_util::future::pending().await;
                        ProcessWaitFailed { err }
                    }
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
                            self.expected_crates = *crate_count.max(&1);
                        }
                        BuildStage::InstallingTooling => {}
                        BuildStage::Compiling { current, total, .. } => {
                            self.compiled_crates = *current;
                            self.expected_crates = *total.max(&1);

                            if self.compile_start.is_none() {
                                self.compile_start = Some(Instant::now());
                            }
                        }
                        BuildStage::Bundling => {
                            self.complete_compile();
                            self.bundling_progress = 0.0;
                            self.bundle_start = Some(Instant::now());
                        }
                        BuildStage::OptimizingWasm => {}
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
                        BuildStage::RunningBindgen => {}
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
            ProcessWaitFailed { .. } => {}
        }

        update
    }

    pub(crate) fn patch_rebuild(&mut self, changed_files: Vec<PathBuf>) {
        // We need the rustc args from the original build to pass to the new build
        let Some(artifacts) = self.artifacts.as_ref().cloned() else {
            tracing::warn!("Ignoring patch rebuild since there is no existing build.");
            return;
        };

        // On web, our patches are fully relocatable, so we don't need to worry about ASLR, but
        // for all other platforms, we need to use the ASLR reference to know where to insert the patch.
        let aslr_reference = match self.aslr_reference {
            Some(val) => val,
            None if self.build.platform == Platform::Web => 0,
            None => {
                tracing::warn!(
                    "Ignoring hotpatch since there is no ASLR reference. Is the client connected?"
                );
                return;
            }
        };

        let cache = artifacts
            .patch_cache
            .clone()
            .context("Failed to get patch cache")
            .unwrap();

        // Abort all the ongoing builds, cleaning up any loose artifacts and waiting to cleanly exit
        self.abort_all(BuildStage::Restarting);
        self.build_task = tokio::spawn({
            let request = self.build.clone();
            let ctx = BuildContext {
                tx: self.tx.clone(),
                mode: BuildMode::Thin {
                    changed_files,
                    rustc_args: artifacts.direct_rustc,
                    aslr_reference,
                    cache,
                },
            };
            async move { request.build(&ctx).await }
        });
    }

    /// Restart this builder with new build arguments.
    pub(crate) fn start_rebuild(&mut self, mode: BuildMode) {
        // Abort all the ongoing builds, cleaning up any loose artifacts and waiting to cleanly exit
        // And then start a new build, resetting our progress/stage to the beginning and replacing the old tokio task
        self.abort_all(BuildStage::Restarting);
        self.artifacts.take();
        self.patch_cache.take();
        self.build_task = tokio::spawn({
            let request = self.build.clone();
            let ctx = BuildContext {
                tx: self.tx.clone(),
                mode,
            };
            async move { request.build(&ctx).await }
        });
    }

    /// Shutdown the current build process
    ///
    /// todo: might want to use a cancellation token here to allow cleaner shutdowns
    pub(crate) fn abort_all(&mut self, stage: BuildStage) {
        self.stage = stage;
        self.compiled_crates = 0;
        self.expected_crates = 1;
        self.bundling_progress = 0.0;
        self.compile_start = None;
        self.bundle_start = None;
        self.bundle_end = None;
        self.compile_end = None;
        self.build_task.abort();
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
                            tracing::info!("Compiled [{current:>3}/{total}]: {krate}");
                        }
                        BuildStage::RunningBindgen => tracing::info!("Running wasm-bindgen..."),
                        BuildStage::CopyingAssets {
                            current,
                            total,
                            path,
                        } => {
                            tracing::info!(
                                "Copying asset ({}/{total}): {}",
                                current + 1,
                                path.display()
                            );
                        }
                        BuildStage::Bundling => tracing::info!("Bundling app..."),
                        _ => {}
                    }

                    tracing::info!(json = ?StructuredOutput::BuildUpdate { stage: stage.clone() });
                }
                BuilderUpdate::CompilerMessage { message } => {
                    tracing::info!(json = ?StructuredOutput::RustcOutput { message: message.clone() }, %message);
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
                            tracing::info!(json = ?StructuredOutput::RustcOutput { message: message.clone() }, %message);
                        }
                    }

                    tracing::error!(?err, json = ?StructuredOutput::Error { message: err.to_string() });
                    return Err(err);
                }
                BuilderUpdate::StdoutReceived { .. } => {}
                BuilderUpdate::StderrReceived { .. } => {}
                BuilderUpdate::ProcessExited { .. } => {}
                BuilderUpdate::ProcessWaitFailed { .. } => {}
            }
        }
    }

    /// Create a list of environment variables that the child process will use
    pub(crate) fn child_environment_variables(
        &mut self,
        devserver_ip: Option<SocketAddr>,
        start_fullstack_on_address: Option<SocketAddr>,
        always_on_top: bool,
        build_id: BuildId,
    ) -> Vec<(&'static str, String)> {
        let krate = &self.build;

        // Set the env vars that the clients will expect
        // These need to be stable within a release version (ie 0.6.0)
        let mut envs = vec![
            (dioxus_cli_config::CLI_ENABLED_ENV, "true".to_string()),
            (
                dioxus_cli_config::APP_TITLE_ENV,
                krate.config.web.app.title.clone(),
            ),
            (
                dioxus_cli_config::SESSION_CACHE_DIR,
                self.build.session_cache_dir().display().to_string(),
            ),
            (dioxus_cli_config::BUILD_ID, build_id.0.to_string()),
            (
                dioxus_cli_config::ALWAYS_ON_TOP_ENV,
                always_on_top.to_string(),
            ),
        ];

        if let Some(devserver_ip) = devserver_ip {
            envs.push((
                dioxus_cli_config::DEVSERVER_IP_ENV,
                devserver_ip.ip().to_string(),
            ));
            envs.push((
                dioxus_cli_config::DEVSERVER_PORT_ENV,
                devserver_ip.port().to_string(),
            ));
        }

        if crate::VERBOSITY
            .get()
            .map(|f| f.verbose)
            .unwrap_or_default()
        {
            envs.push(("RUST_BACKTRACE", "1".to_string()));
        }

        if let Some(base_path) = krate.base_path() {
            envs.push((dioxus_cli_config::ASSET_ROOT_ENV, base_path.to_string()));
        }

        if let Some(env_filter) = env::var_os("RUST_LOG").and_then(|e| e.into_string().ok()) {
            envs.push(("RUST_LOG", env_filter));
        }

        // Launch the server if we were given an address to start it on, and the build includes a server. After we
        // start the server, consume its stdout/stderr.
        if let Some(addr) = start_fullstack_on_address {
            envs.push((dioxus_cli_config::SERVER_IP_ENV, addr.ip().to_string()));
            envs.push((dioxus_cli_config::SERVER_PORT_ENV, addr.port().to_string()));
        }

        envs
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn open(
        &mut self,
        devserver_ip: SocketAddr,
        open_address: Option<SocketAddr>,
        start_fullstack_on_address: Option<SocketAddr>,
        open_browser: bool,
        always_on_top: bool,
        build_id: BuildId,
        args: &[String],
    ) -> Result<()> {
        let envs = self.child_environment_variables(
            Some(devserver_ip),
            start_fullstack_on_address,
            always_on_top,
            build_id,
        );

        // We try to use stdin/stdout to communicate with the app
        match self.build.platform {
            // Unfortunately web won't let us get a proc handle to it (to read its stdout/stderr) so instead
            // use use the websocket to communicate with it. I wish we could merge the concepts here,
            // like say, opening the socket as a subprocess, but alas, it's simpler to do that somewhere else.
            Platform::Web => {
                // Only the first build we open the web app, after that the user knows it's running
                if open_browser {
                    self.open_web(open_address.unwrap_or(devserver_ip));
                }
            }

            Platform::Ios => self.open_ios_sim(envs).await?,

            Platform::Android => {
                self.open_android_sim(false, devserver_ip, envs).await?;
            }

            // These are all just basically running the main exe, but with slightly different resource dir paths
            Platform::Server
            | Platform::MacOS
            | Platform::Windows
            | Platform::Linux
            | Platform::Liveview => self.open_with_main_exe(envs, args)?,
        };

        self.builds_opened += 1;

        Ok(())
    }

    /// Gracefully kill the process and all of its children
    ///
    /// Uses the `SIGTERM` signal on unix and `taskkill` on windows.
    /// This complex logic is necessary for things like window state preservation to work properly.
    ///
    /// Also wipes away the entropy executables if they exist.
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

        // Wipe out the entropy executables if they exist
        if let Some(entropy_app_exe) = self.entropy_app_exe.take() {
            _ = std::fs::remove_file(entropy_app_exe);
        }
    }

    pub(crate) async fn hotpatch(
        &mut self,
        res: &BuildArtifacts,
        cache: &HotpatchModuleCache,
    ) -> Result<JumpTable> {
        let original = self.build.main_exe();
        let new = self.build.patch_exe(res.time_start);
        let triple = self.build.triple.clone();
        let original_artifacts = self.artifacts.as_ref().unwrap();
        let asset_dir = self.build.asset_dir();

        for bundled in res.assets.assets() {
            if original_artifacts.assets.contains(bundled) {
                continue;
            }
            let from = dunce::canonicalize(PathBuf::from(bundled.absolute_source_path()))?;

            let to = asset_dir.join(bundled.bundled_path());

            tracing::debug!("Copying asset from patch: {}", from.display());
            if let Err(e) = dioxus_cli_opt::process_file_to(bundled.options(), &from, &to) {
                tracing::error!("Failed to copy asset: {e}");
                continue;
            }

            // If the emulator is android, we need to copy the asset to the device with `adb push asset /data/local/tmp/dx/assets/filename.ext`
            if self.build.platform == Platform::Android {
                let bundled_name = PathBuf::from(bundled.bundled_path());
                _ = self.copy_file_to_android_tmp(&from, &bundled_name).await;
            }
        }

        tracing::debug!("Patching {} -> {}", original.display(), new.display());

        let mut jump_table = crate::build::create_jump_table(&new, &triple, cache)?;

        // If it's android, we need to copy the assets to the device and then change the location of the patch
        if self.build.platform == Platform::Android {
            jump_table.lib = self
                .copy_file_to_android_tmp(&new, &(PathBuf::from(new.file_name().unwrap())))
                .await?;
        }

        // Rebase the wasm binary to be relocatable once the jump table is generated
        if triple.architecture == target_lexicon::Architecture::Wasm32 {
            // Make sure we use the dir relative to the public dir, so the web can load it as a proper URL
            //
            // ie we would've shipped `/Users/foo/Projects/dioxus/target/dx/project/debug/web/public/wasm/lib.wasm`
            //    but we want to ship `/wasm/lib.wasm`
            jump_table.lib =
                PathBuf::from("/").join(jump_table.lib.strip_prefix(self.build.root_dir()).unwrap())
        }

        let changed_files = match &res.mode {
            BuildMode::Thin { changed_files, .. } => changed_files.clone(),
            _ => vec![],
        };

        use crate::styles::{GLOW_STYLE, NOTE_STYLE};

        let changed_file = changed_files.first().unwrap();
        tracing::info!(
            "Hot-patching: {NOTE_STYLE}{}{NOTE_STYLE:#} took {GLOW_STYLE}{:?}ms{GLOW_STYLE:#}",
            changed_file
                .display()
                .to_string()
                .trim_start_matches(&self.build.crate_dir().display().to_string()),
            SystemTime::now()
                .duration_since(res.time_start)
                .unwrap()
                .as_millis()
        );

        self.patches.push(jump_table.clone());

        Ok(jump_table)
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
    /// This will return the bundled name of the assets such that we can send it to the clients letting
    /// them know what to reload. It's not super important that this is robust since most clients will
    /// kick all stylsheets without necessarily checking the name.
    pub(crate) async fn hotreload_bundled_assets(
        &self,
        changed_file: &PathBuf,
    ) -> Option<Vec<PathBuf>> {
        let artifacts = self.artifacts.as_ref()?;

        // Use the build dir if there's no runtime asset dir as the override. For the case of ios apps,
        // we won't actually be using the build dir.
        let asset_dir = match self.runtime_asset_dir.as_ref() {
            Some(dir) => dir.to_path_buf().join("assets/"),
            None => self.build.asset_dir(),
        };

        // Canonicalize the path as Windows may use long-form paths "\\\\?\\C:\\".
        let changed_file = dunce::canonicalize(changed_file)
            .inspect_err(|e| tracing::debug!("Failed to canonicalize hotreloaded asset: {e}"))
            .ok()?;

        // The asset might've been renamed thanks to the manifest, let's attempt to reload that too
        let resources = artifacts.assets.get_assets_for_source(&changed_file)?;
        let mut bundled_names = Vec::new();
        for resource in resources {
            let output_path = asset_dir.join(resource.bundled_path());

            tracing::debug!("Hotreloading asset {changed_file:?} in target {asset_dir:?}");

            // Remove the old asset if it exists
            _ = std::fs::remove_file(&output_path);

            // And then process the asset with the options into the **old** asset location. If we recompiled,
            // the asset would be in a new location because the contents and hash have changed. Since we are
            // hotreloading, we need to use the old asset location it was originally written to.
            let options = *resource.options();
            let res = process_file_to(&options, &changed_file, &output_path);
            let bundled_name = PathBuf::from(resource.bundled_path());
            if let Err(e) = res {
                tracing::debug!("Failed to hotreload asset {e}");
            }

            // If the emulator is android, we need to copy the asset to the device with `adb push asset /data/local/tmp/dx/assets/filename.ext`
            if self.build.platform == Platform::Android {
                _ = self
                    .copy_file_to_android_tmp(&changed_file, &bundled_name)
                    .await;
            }
            bundled_names.push(bundled_name);
        }

        Some(bundled_names)
    }

    /// Copy this file to the tmp folder on the android device, returning the path to the copied file
    ///
    /// When we push patches (.so), the runtime will dlopen the file from the tmp folder by first copying
    /// it to shared memory. This is a workaround since not all android devices will be rooted and we
    /// can't drop the file into the `/data/data/com.org.app/lib/` directory.
    pub(crate) async fn copy_file_to_android_tmp(
        &self,
        changed_file: &Path,
        bundled_name: &Path,
    ) -> Result<PathBuf> {
        let target = dioxus_cli_config::android_session_cache_dir().join(bundled_name);
        tracing::debug!("Pushing asset to device: {target:?}");

        let res = Command::new(&self.build.workspace.android_tools()?.adb)
            .arg("push")
            .arg(changed_file)
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
    fn open_with_main_exe(&mut self, envs: Vec<(&str, String)>, args: &[String]) -> Result<()> {
        let main_exe = self.app_exe();

        tracing::debug!("Opening app with main exe: {main_exe:?}");

        let mut child = Command::new(main_exe)
            .args(args)
            .envs(envs)
            .env_remove("CARGO_MANIFEST_DIR") // running under `dx` shouldn't expose cargo-only :
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stderr = BufReader::new(child.stderr.take().unwrap());
        self.stdout = Some(stdout.lines());
        self.stderr = Some(stderr.lines());
        self.child = Some(child);

        Ok(())
    }

    /// Open the web app by opening the browser to the given address.
    /// Check if we need to use https or not, and if so, add the protocol.
    /// Go to the basepath if that's set too.
    fn open_web(&self, address: SocketAddr) {
        let base_path = self.build.base_path();
        let https = self.build.config.web.https.enabled.unwrap_or_default();
        let protocol = if https { "https" } else { "http" };
        let base_path = match base_path {
            Some(base_path) => format!("/{}", base_path.trim_matches('/')),
            None => "".to_owned(),
        };
        _ = open::that_detached(format!("{protocol}://{address}{base_path}"));
    }

    /// Use `xcrun` to install the app to the simulator
    /// With simulators, we're free to basically do anything, so we don't need to do any fancy codesigning
    /// or entitlements, or anything like that.
    ///
    /// However, if there's no simulator running, this *might* fail.
    ///
    /// TODO(jon): we should probably check if there's a simulator running before trying to install,
    /// and open the simulator if we have to.
    async fn open_ios_sim(&mut self, envs: Vec<(&str, String)>) -> Result<()> {
        tracing::debug!("Installing app to simulator {:?}", self.build.root_dir());

        let res = Command::new("xcrun")
            .arg("simctl")
            .arg("install")
            .arg("booted")
            .arg(self.build.root_dir())
            .output()
            .await?;

        tracing::debug!("Installed app to simulator with exit code: {res:?}");

        // Remap the envs to the correct simctl env vars
        // iOS sim lets you pass env vars but they need to be in the format "SIMCTL_CHILD_XXX=XXX"
        let ios_envs = envs
            .iter()
            .map(|(k, v)| (format!("SIMCTL_CHILD_{k}"), v.clone()));

        let mut child = Command::new("xcrun")
            .arg("simctl")
            .arg("launch")
            .arg("--console")
            .arg("booted")
            .arg(self.build.bundle_identifier())
            .envs(ios_envs)
            .env_remove("CARGO_MANIFEST_DIR")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stderr = BufReader::new(child.stderr.take().unwrap());
        self.stdout = Some(stdout.lines());
        self.stderr = Some(stderr.lines());
        self.child = Some(child);

        Ok(())
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
        // 1231231231231asdasdads123123 "Apple Development: foo@gmail.com (XYZYZY)"
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

    /// Launch the Android simulator and deploy the application.
    ///
    /// This function handles the process of starting the Android simulator, installing the APK,
    /// forwarding the development server port, and launching the application on the simulator.
    ///
    /// The following `adb` commands are executed:
    ///
    /// 1. **Enable Root Access**:
    ///    - `adb root`: Enables root access on the Android simulator, allowing for advanced operations like pushing files to restricted directories.
    ///
    /// 2. **Port Forwarding**:
    ///    - `adb reverse tcp:<port> tcp:<port>`: Forwards the development server port from the host
    ///      machine to the Android simulator, enabling communication between the app and the dev server.
    ///
    /// 3. **APK Installation**:
    ///    - `adb install -r <apk_path>`: Installs the APK onto the Android simulator. The `-r` flag
    ///      ensures that any existing installation of the app is replaced.
    ///
    /// 4. **Environment Variables**:
    ///    - Writes environment variables to a `.env` file in the session cache directory.
    ///    - `adb push <local_env_file> <device_env_file>`: Pushes the `.env` file to the Android device
    ///      to configure runtime environment variables for the app.
    ///
    /// 5. **App Launch**:
    ///    - `adb shell am start -n <package_name>/<activity_name>`: Launches the app on the Android
    ///      simulator. The `<package_name>` and `<activity_name>` are derived from the app's configuration.
    ///
    /// # Notes
    ///
    /// - This function is asynchronous and spawns a background task to handle the simulator setup and app launch.
    /// - The Android tools (`adb`) must be available in the system's PATH for this function to work.
    /// - If the app fails to launch, errors are logged for debugging purposes.
    ///
    /// # Resources:
    /// - <https://developer.android.com/studio/run/emulator-commandline>
    async fn open_android_sim(
        &self,
        root: bool,
        devserver_socket: SocketAddr,
        envs: Vec<(&'static str, String)>,
    ) -> Result<()> {
        let apk_path = self.build.debug_apk_path();
        let session_cache = self.build.session_cache_dir();
        let application_id = self.build.bundle_identifier();
        let adb = self.build.workspace.android_tools()?.adb.clone();

        // Start backgrounded since .open() is called while in the arm of the top-level match
        let _handle: JoinHandle<Result<()>> = tokio::task::spawn(async move {
            // call `adb root` so we can push patches to the device
            if root {
                if let Err(e) = Command::new(&adb).arg("root").output().await {
                    tracing::error!("Failed to run `adb root`: {e}");
                }
            }

            let port = devserver_socket.port();
            if let Err(e) = Command::new(&adb)
                .arg("reverse")
                .arg(format!("tcp:{}", port))
                .arg(format!("tcp:{}", port))
                .output()
                .await
            {
                tracing::error!("failed to forward port {port}: {e}");
            }

            // Wait for device to be ready
            let cmd = Command::new(&adb)
                .arg("wait-for-device")
                .arg("shell")
                .arg(r#"while [[ -z $(getprop sys.boot_completed) ]]; do sleep 1; done;"#)
                .output();
            let cmd_future = cmd.fuse();
            pin_mut!(cmd_future);
            tokio::select! {
                _ = &mut cmd_future => {}
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    tracing::info!("Waiting for android emulator to be ready...");
                    _ = cmd_future.await;
                }
            }

            // Install
            // adb install -r app-debug.apk
            let res = Command::new(&adb)
                .arg("install")
                .arg("-r")
                .arg(apk_path)
                .output()
                .await?;
            let std_err = String::from_utf8_lossy(&res.stderr);
            if !std_err.is_empty() {
                tracing::error!("Failed to install apk with `adb`: {std_err}");
            }

            // Clear the session cache dir on the device
            Command::new(&adb)
                .arg("shell")
                .arg("rm")
                .arg("-rf")
                .arg(dioxus_cli_config::android_session_cache_dir())
                .output()
                .await?;

            // Write the env vars to a .env file in our session cache
            let env_file = session_cache.join(".env");
            _ = std::fs::write(
                &env_file,
                envs.iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );

            // Push the env file to the device
            Command::new(&adb)
                .arg("push")
                .arg(env_file)
                .arg(dioxus_cli_config::android_session_cache_dir().join(".env"))
                .output()
                .await?;

            // eventually, use the user's MainActivity, not our MainActivity
            // adb shell am start -n dev.dioxus.main/dev.dioxus.main.MainActivity
            let activity_name = format!("{application_id}/dev.dioxus.main.MainActivity");
            let res = Command::new(&adb)
                .arg("shell")
                .arg("am")
                .arg("start")
                .arg("-n")
                .arg(activity_name)
                .output()
                .await?;
            let std_err = String::from_utf8_lossy(res.stderr.trim_ascii());
            if !std_err.is_empty() {
                tracing::error!("Failed to start app with `adb`: {std_err}");
            }

            Ok(())
        });

        Ok(())
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

        if requires_entropy || crate::devcfg::should_force_entropy() {
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
        matches!(&self.stage, BuildStage::Success | BuildStage::Failed)
    }

    pub(crate) async fn open_debugger(&mut self, server: &WebServer) -> Result<()> {
        let url = match self.build.platform {
            Platform::MacOS
            | Platform::Windows
            | Platform::Linux
            | Platform::Server
            | Platform::Liveview => {
                let Some(Some(pid)) = self.child.as_mut().map(|f| f.id()) else {
                    tracing::warn!("No process to attach debugger to");
                    return Ok(());
                };

                format!(
                    "vscode://vadimcn.vscode-lldb/launch/config?{{'request':'attach','pid':{}}}",
                    pid
                )
            }

            Platform::Web => {
                // code --open-url "vscode://DioxusLabs.dioxus/debugger?uri=http://127.0.0.1:8080"
                // todo - debugger could open to the *current* page afaik we don't have a way to have that info
                let address = server.devserver_address();
                let base_path = self.build.base_path();
                let https = self.build.config.web.https.enabled.unwrap_or_default();
                let protocol = if https { "https" } else { "http" };
                let base_path = match base_path {
                    Some(base_path) => format!("/{}", base_path.trim_matches('/')),
                    None => "".to_owned(),
                };
                format!("vscode://DioxusLabs.dioxus/debugger?uri={protocol}://{address}{base_path}")
            }

            Platform::Ios => {
                let Some(pid) = self.pid else {
                    tracing::warn!("No process to attach debugger to");
                    return Ok(());
                };

                format!(
                    "vscode://vadimcn.vscode-lldb/launch/config?{{'request':'attach','pid':{pid}}}"
                )
            }

            // https://stackoverflow.com/questions/53733781/how-do-i-use-lldb-to-debug-c-code-on-android-on-command-line/64997332#64997332
            // https://android.googlesource.com/platform/development/+/refs/heads/main/scripts/gdbclient.py
            // run lldbserver on the device and then connect
            //
            // # TODO: https://code.visualstudio.com/api/references/vscode-api#debug and
            // #       https://code.visualstudio.com/api/extension-guides/debugger-extension and
            // #       https://github.com/vadimcn/vscode-lldb/blob/6b775c439992b6615e92f4938ee4e211f1b060cf/extension/pickProcess.ts#L6
            //
            // res = {
            //     "name": "(lldbclient.py) Attach {} (port: {})".format(binary_name.split("/")[-1], port),
            //     "type": "lldb",
            //     "request": "custom",
            //     "relativePathBase": root,
            //     "sourceMap": { "/b/f/w" : root, '': root, '.': root },
            //     "initCommands": ['settings append target.exec-search-paths {}'.format(' '.join(solib_search_path))],
            //     "targetCreateCommands": ["target create {}".format(binary_name),
            //                              "target modules search-paths add / {}/".format(sysroot)],
            //     "processCreateCommands": ["gdb-remote {}".format(str(port))]
            // }
            //
            // https://github.com/vadimcn/codelldb/issues/213
            //
            // lots of pain to figure this out:
            //
            // (lldb) image add target/dx/tw6/debug/android/app/app/src/main/jniLibs/arm64-v8a/libdioxusmain.so
            // (lldb) settings append target.exec-search-paths target/dx/tw6/debug/android/app/app/src/main/jniLibs/arm64-v8a/libdioxusmain.so
            // (lldb) process handle SIGSEGV --pass true --stop false --notify true (otherwise the java threads cause crash)
            //
            Platform::Android => {
                // adb push ./sdk/ndk/29.0.13113456/toolchains/llvm/prebuilt/darwin-x86_64/lib/clang/20/lib/linux/aarch64/lldb-server /tmp
                // adb shell "/tmp/lldb-server --server --listen ..."
                // "vscode://vadimcn.vscode-lldb/launch/config?{{'request':'connect','port': {}}}",
                // format!(
                //     "vscode://vadimcn.vscode-lldb/launch/config?{{'request':'attach','pid':{pid}}}"
                // )
                let tools = &self.build.workspace.android_tools()?;

                // get the pid of the app
                let pid = Command::new(&tools.adb)
                    .arg("shell")
                    .arg("pidof")
                    .arg(self.build.bundle_identifier())
                    .output()
                    .await
                    .ok()
                    .and_then(|output| String::from_utf8(output.stdout).ok())
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap();

                // copy the lldb-server to the device
                let lldb_server = tools
                    .android_tools_dir()
                    .parent()
                    .unwrap()
                    .join("lib")
                    .join("clang")
                    .join("20")
                    .join("lib")
                    .join("linux")
                    .join("aarch64")
                    .join("lldb-server");

                tracing::info!("Copying lldb-server to device: {lldb_server:?}");

                _ = Command::new(&tools.adb)
                    .arg("push")
                    .arg(lldb_server)
                    .arg("/tmp/lldb-server")
                    .output()
                    .await;

                // Forward requests on 10086 to the device
                _ = Command::new(&tools.adb)
                    .arg("forward")
                    .arg("tcp:10086")
                    .arg("tcp:10086")
                    .output()
                    .await;

                // start the server - running it multiple times will make the subsequent ones fail (which is fine)
                _ = Command::new(&tools.adb)
                    .arg("shell")
                    .arg(r#"cd /tmp && ./lldb-server platform --server --listen '*:10086'"#)
                    .kill_on_drop(false)
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn();

                let program_path = self.build.main_exe();
                format!(
                    r#"vscode://vadimcn.vscode-lldb/launch/config?{{
                        'name':'Attach to Android',
                        'type':'lldb',
                        'request':'attach',
                        'pid': '{pid}',
                        'processCreateCommands': [
                            'platform select remote-android',
                            'platform connect connect://localhost:10086',
                            'settings set target.inherit-env false',
                            'settings set target.inline-breakpoint-strategy always',
                            'settings set target.process.thread.step-avoid-regexp \"JavaBridge|JDWP|Binder|ReferenceQueueDaemon\"',
                            'process handle SIGSEGV --pass true --stop false --notify true"',
                            'settings append target.exec-search-paths {program_path}',
                            'attach --pid {pid}',
                            'continue'
                        ]
                    }}"#,
                    program_path = program_path.display(),
                )
                .lines()
                .map(|line| line.trim())
                .join("")
            }
        };

        tracing::info!("Opening debugger for [{}]: {url}", self.build.platform);

        _ = tokio::process::Command::new("code")
            .arg("--open-url")
            .arg(url)
            .spawn();

        Ok(())
    }
}
