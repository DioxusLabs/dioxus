use crate::{
    build::cache::ObjectCache, serve::WebServer, verbosity_or_default, BuildArtifacts,
    BuildRequest, BuildStage, BuilderUpdate, BundleFormat, ProgressRx, ProgressTx, Result,
    RustcArgs, StructuredOutput,
};
use anyhow::{bail, Context, Error};
use dioxus_cli_opt::process_file_to;
use futures_util::{future::OptionFuture, pin_mut, FutureExt};
use itertools::Itertools;
use std::{
    collections::HashSet,
    env,
    time::{Duration, Instant, SystemTime},
};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Stdio,
};
use subsecond_types::JumpTable;
use target_lexicon::Architecture;
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{Child, ChildStderr, ChildStdout, Command},
    task::JoinHandle,
};
use tokio_stream::wrappers::UnboundedReceiverStream;

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

    // Android logcat stream (treated as stderr for error/warn levels)
    pub adb_logcat_stdout: Option<UnboundedReceiverStream<String>>,

    /// Handle to the task that's monitoring the child process
    pub spawn_handle: Option<JoinHandle<Result<()>>>,

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

    /// Cumulative set of workspace crates modified since the last fat build.
    /// Each patch includes objects from ALL crates in this set.
    pub modified_crates: HashSet<String>,

    /// Cache of the latest `.rcgu.o` files for each modified workspace crate.
    pub object_cache: ObjectCache,
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
    pub(crate) fn new(request: &BuildRequest) -> Result<Self> {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        Ok(Self {
            build: request.clone(),
            stage: BuildStage::Initializing,
            build_task: tokio::task::spawn(std::future::pending()),
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
            adb_logcat_stdout: None,
            spawn_handle: None,
            entropy_app_exe: None,
            artifacts: None,
            patch_cache: None,
            pid: None,
            modified_crates: HashSet::new(),
            object_cache: ObjectCache::new(&request.session_cache_dir()),
        })
    }

    /// Create a new `AppBuilder` and immediately start a build process.
    pub fn started(request: &BuildRequest, mode: BuildMode, build_id: BuildId) -> Result<Self> {
        let mut builder = Self::new(request)?;
        builder.start(mode, build_id);
        Ok(builder)
    }

    pub(crate) fn start(&mut self, mode: BuildMode, build_id: BuildId) {
        self.build_task = tokio::spawn({
            let request = self.build.clone();
            let tx = self.tx.clone();
            async move {
                let ctx = BuildContext {
                    mode,
                    build_id,
                    tx: tx.clone(),
                };
                request.verify_tooling(&ctx).await?;
                request.prebuild(&ctx).await?;
                request.build(&ctx).await
            }
        });
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
                    Err(err) => BuilderUpdate::BuildFailed { err: anyhow::anyhow!("Build panicked! {err:#?}") },
                }
            },
            Some(Ok(Some(msg))) = OptionFuture::from(self.stdout.as_mut().map(|f| f.next_line())) => {
                StdoutReceived {  msg }
            },
            Some(Ok(Some(msg))) = OptionFuture::from(self.stderr.as_mut().map(|f| f.next_line())) => {
                StderrReceived {  msg }
            },
            Some(msg) = OptionFuture::from(self.spawn_handle.as_mut()) => {
                match msg {
                    Ok(Ok(_)) => StdoutReceived { msg: "Finished launching app".to_string() },
                    Ok(Err(err)) => StderrReceived { msg: err.to_string() },
                    Err(err) => StderrReceived { msg: err.to_string() }
                }
            },
            Some(Some(msg)) = OptionFuture::from(self.adb_logcat_stdout.as_mut().map(|s| s.next())) => {
                // Send as stderr for errors/warnings, stdout for info/debug
                // Parse the priority level from a logcat line
                //
                // Logcat brief format: "I/TAG(12345): message"
                // Returns the priority char (V, D, I, W, E, F)
                if matches!(msg.chars().next().unwrap_or('I'), 'E' | 'W' | 'F') {
                    StderrReceived { msg }
                } else {
                    StdoutReceived { msg }
                }
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

    pub(crate) fn patch_rebuild(
        &mut self,
        changed_files: Vec<PathBuf>,
        changed_crates: Vec<String>,
        build_id: BuildId,
    ) {
        // We need the rustc args from the original build to pass to the new build
        let Some(artifacts) = self.artifacts.as_ref().cloned() else {
            tracing::warn!(
                "Ignoring patch rebuild for {build_id:?} since there is no existing build."
            );
            return;
        };

        // On web, our patches are fully relocatable, so we don't need to worry about ASLR, but
        // for all other platforms, we need to use the ASLR reference to know where to insert the patch.
        let aslr_reference = match self.aslr_reference {
            Some(val) => val,
            None if matches!(
                self.build.triple.architecture,
                Architecture::Wasm32 | Architecture::Wasm64
            ) =>
            {
                0
            }
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

        // Add the changed crates to the cumulative modified set.
        // Every patch includes objects from ALL crates that have been modified since the fat build.
        let tip_crate_name = self.build.main_target.replace('-', "_");
        for crate_name in &changed_crates {
            self.modified_crates.insert(crate_name.clone());
        }

        // The tip crate is always in the modified set since we always relink it.
        // (It might not need recompilation if assembly diff shows no cascade, but
        // its objects must be in the patch dylib.)
        self.modified_crates.insert(tip_crate_name);

        tracing::debug!(
            "Patch rebuild: changed_crates={:?}, modified_crates={:?}",
            changed_crates,
            self.modified_crates,
        );

        // Abort all the ongoing builds, cleaning up any loose artifacts and waiting to cleanly exit
        self.abort_all(BuildStage::Restarting);
        self.build_task = tokio::spawn({
            let request = self.build.clone();
            let ctx = BuildContext {
                build_id,
                tx: self.tx.clone(),
                mode: BuildMode::Thin {
                    changed_files,
                    changed_crates,
                    modified_crates: self.modified_crates.clone(),
                    workspace_rustc_args: artifacts.workspace_rustc_args,
                    aslr_reference,
                    cache,
                    object_cache: self.object_cache.clone(),
                },
            };
            async move { request.build(&ctx).await }
        });
    }

    /// Restart this builder with new build arguments.
    pub(crate) fn start_rebuild(&mut self, mode: BuildMode, build_id: BuildId) {
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
                build_id,
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
                        BuildStage::CodeSigning => tracing::info!("Code signing app..."),
                        _ => {}
                    }

                    tracing::info!(json = %StructuredOutput::BuildUpdate { stage: stage.clone() });
                }
                BuilderUpdate::CompilerMessage { message } => {
                    tracing::info!(json = %StructuredOutput::RustcOutput { message: message.clone() }, %message);
                }
                BuilderUpdate::BuildReady { bundle } => {
                    tracing::debug!(json = %StructuredOutput::BuildFinished {
                        artifacts: bundle.clone().into_structured_output(),
                    });
                    return Ok(bundle);
                }
                BuilderUpdate::BuildFailed { err } => {
                    // Flush remaining compiler messages
                    while let Ok(Some(msg)) = self.rx.try_next() {
                        if let BuilderUpdate::CompilerMessage { message } = msg {
                            tracing::info!(json = %StructuredOutput::RustcOutput { message: message.clone() }, %message);
                        }
                    }

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
    ///
    /// We try to emulate running under `cargo` as much as possible, carrying over vars like `CARGO_MANIFEST_DIR`.
    /// Previously, we didn't want to emulate this behavior, but now we do in order to be a good
    /// citizen of the Rust ecosystem and allow users to use `cargo` features like `CARGO_MANIFEST_DIR`.
    ///
    /// Note that Dioxus apps *should not* rely on this vars being set, but libraries like Bevy do.
    pub(crate) fn child_environment_variables(
        &mut self,
        devserver_ip: Option<SocketAddr>,
        start_fullstack_on_address: Option<SocketAddr>,
        always_on_top: bool,
        build_id: BuildId,
    ) -> Vec<(String, String)> {
        let krate = &self.build;

        // Set the env vars that the clients will expect
        // These need to be stable within a release version (ie 0.6.0)
        let mut envs: Vec<(String, String)> = vec![
            (
                dioxus_cli_config::CLI_ENABLED_ENV.into(),
                "true".to_string(),
            ),
            (
                dioxus_cli_config::APP_TITLE_ENV.into(),
                krate.config.web.app.title.clone(),
            ),
            (
                dioxus_cli_config::SESSION_CACHE_DIR.into(),
                self.build.session_cache_dir().display().to_string(),
            ),
            (dioxus_cli_config::BUILD_ID.into(), build_id.0.to_string()),
            (
                dioxus_cli_config::ALWAYS_ON_TOP_ENV.into(),
                always_on_top.to_string(),
            ),
        ];

        if let Some(devserver_ip) = devserver_ip {
            envs.push((
                dioxus_cli_config::DEVSERVER_IP_ENV.into(),
                devserver_ip.ip().to_string(),
            ));
            envs.push((
                dioxus_cli_config::DEVSERVER_PORT_ENV.into(),
                devserver_ip.port().to_string(),
            ));
        }

        if verbosity_or_default().verbose {
            envs.push(("RUST_BACKTRACE".into(), "1".to_string()));
        }

        if let Some(base_path) = krate.trimmed_base_path() {
            envs.push((
                dioxus_cli_config::ASSET_ROOT_ENV.into(),
                base_path.to_string(),
            ));
        }

        if let Some(env_filter) = env::var_os("RUST_LOG").and_then(|e| e.into_string().ok()) {
            envs.push(("RUST_LOG".into(), env_filter));
        }

        // Launch the server if we were given an address to start it on, and the build includes a server. After we
        // start the server, consume its stdout/stderr.
        if let Some(addr) = start_fullstack_on_address {
            envs.push((
                dioxus_cli_config::SERVER_IP_ENV.into(),
                addr.ip().to_string(),
            ));
            envs.push((
                dioxus_cli_config::SERVER_PORT_ENV.into(),
                addr.port().to_string(),
            ));
        }

        // If there's any CARGO vars in the rustc_wrapper files, push those too.
        // Read from any per-crate args file in the directory (they all share the same CARGO_ envs).
        if let Ok(entries) = std::fs::read_dir(self.build.rustc_wrapper_args_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json") {
                    if let Ok(contents) = std::fs::read_to_string(&path) {
                        if let Ok(args) = serde_json::from_str::<RustcArgs>(&contents) {
                            for (key, value) in args.envs {
                                if key.starts_with("CARGO_") {
                                    envs.push((key, value));
                                }
                            }
                            break; // Only need one file for CARGO_ env vars
                        }
                    }
                }
            }
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
        match self.build.bundle {
            // Unfortunately web won't let us get a proc handle to it (to read its stdout/stderr) so instead
            // use use the websocket to communicate with it. I wish we could merge the concepts here,
            // like say, opening the socket as a subprocess, but alas, it's simpler to do that somewhere else.
            BundleFormat::Web => {
                // Only the first build we open the web app, after that the user knows it's running
                if open_browser {
                    self.open_web(open_address.unwrap_or(devserver_ip));
                }
            }

            BundleFormat::Ios => {
                if let Some(device) = self.build.device_name.to_owned() {
                    self.open_ios_device(&device).await?
                } else {
                    self.open_ios_sim(envs).await?
                }
            }

            BundleFormat::Android => {
                self.open_android(false, devserver_ip, envs, self.build.device_name.clone())
                    .await?;
            }

            // These are all just basically running the main exe, but with slightly different resource dir paths
            BundleFormat::Server
            | BundleFormat::MacOS
            | BundleFormat::Windows
            | BundleFormat::Linux => self.open_with_main_exe(envs, args)?,
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
                .args(["/PID", &pid.to_string()])
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

        // Abort the spawn handle monitoring task if it exists
        if let Some(spawn_handle) = self.spawn_handle.take() {
            spawn_handle.abort();
        }
    }

    pub(crate) async fn hotpatch(
        &mut self,
        res: &BuildArtifacts,
        cache: &HotpatchModuleCache,
    ) -> Result<JumpTable> {
        let original = self.build.main_exe();
        let new = self.build.patch_exe(res.time_start);
        let asset_dir = self.build.asset_dir();

        // Hotpatch asset!() calls
        for bundled in res.assets.unique_assets() {
            let original_artifacts = self
                .artifacts
                .as_mut()
                .context("No artifacts to hotpatch")?;

            if original_artifacts.assets.contains(bundled) {
                continue;
            }

            // If this is a new asset, insert it into the artifacts so we can track it when hot reloading
            original_artifacts.assets.insert_asset(*bundled);

            let from = dunce::canonicalize(PathBuf::from(bundled.absolute_source_path()))?;

            let to = asset_dir.join(bundled.bundled_path());

            tracing::debug!("Copying asset from patch: {}", from.display());
            if let Err(e) = dioxus_cli_opt::process_file_to(bundled.options(), &from, &to) {
                tracing::error!("Failed to copy asset: {e}");
                continue;
            }

            // If the emulator is android, we need to copy the asset to the device with `adb push asset /data/local/tmp/dx/assets/filename.ext`
            if self.build.bundle == BundleFormat::Android {
                let bundled_name = PathBuf::from(bundled.bundled_path());
                _ = self.copy_file_to_android_tmp(&from, &bundled_name).await;
            }
        }

        // Make sure to add `include!()` calls to the watcher so we can watch changes as they evolve
        for file in res.depinfo.files.iter() {
            let original_artifacts = self
                .artifacts
                .as_mut()
                .context("No artifacts to hotpatch")?;

            if !original_artifacts.depinfo.files.contains(file) {
                original_artifacts.depinfo.files.push(file.clone());
            }
        }

        tracing::debug!("Patching {} -> {}", original.display(), new.display());

        let mut jump_table = self.build.create_jump_table(&new, cache)?;

        // If it's android, we need to copy the assets to the device and then change the location of the patch
        if self.build.bundle == BundleFormat::Android {
            jump_table.lib = self
                .copy_file_to_android_tmp(&new, &(PathBuf::from(new.file_name().unwrap())))
                .await?;
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

        // Sync the updated object cache and modified crates back from the build artifacts.
        // The object files we link with will have changed.
        self.object_cache = res.object_cache.clone();
        self.modified_crates = res.modified_crates.clone();

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
            if self.build.bundle == BundleFormat::Android {
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
    fn open_with_main_exe(&mut self, envs: Vec<(String, String)>, args: &[String]) -> Result<()> {
        let main_exe = self.app_exe();

        tracing::debug!("Opening app with main exe: {main_exe:?}");

        let mut child = Command::new(main_exe)
            .args(args)
            .envs(envs)
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
    async fn open_ios_sim(&mut self, envs: Vec<(String, String)>) -> Result<()> {
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

    /// Upload the app to the device and launch it
    async fn open_ios_device(&mut self, device_query: &str) -> Result<()> {
        let device_query = device_query.to_string();
        let root_dir = self.build.root_dir().clone();
        self.spawn_handle = Some(tokio::task::spawn(async move {
            // 1. Find an active device
            let device_uuid = Self::get_ios_device_uuid(&device_query).await?;

            tracing::info!("Uploading app to iOS device, this might take a while...");

            // 2. Get the installation URL of the app
            let installation_url = Self::get_ios_installation_url(&device_uuid, &root_dir).await?;

            // 3. Launch the app into the background, paused
            Self::launch_ios_app_paused(&device_uuid, &installation_url).await?;

            Result::Ok(()) as Result<()>
        }));

        Ok(())
    }

    /// Parse the xcrun output to get the device based on its name and connected state.
    ///
    /// ```json, ignore
    /// "connectionProperties" : {
    ///   "authenticationType" : "manualPairing",
    ///   "isMobileDeviceOnly" : false,
    ///   "lastConnectionDate" : "2025-08-15T01:46:43.182Z",
    ///   "pairingState" : "paired",
    ///   "potentialHostnames" : [
    ///     "00008130-0002058401E8001C.coredevice.local",
    ///     "67054C13-C6C8-5AC2-B967-24C040AD3F17.coredevice.local"
    ///   ],
    ///   "transportType" : "localNetwork",
    ///   "tunnelState" : "disconnected",
    ///   "tunnelTransportProtocol" : "tcp"
    /// },
    /// "deviceProperties" : {
    ///   "bootedFromSnapshot" : true,
    ///   "bootedSnapshotName" : "com.apple.os.update-A771E2B3E8C155D1B1188896B3247851B64737ACDE91A5B6F6C1F03A541406AA",
    ///   "ddiServicesAvailable" : false,
    ///   "developerModeStatus" : "enabled",
    ///   "hasInternalOSBuild" : false,
    ///   "name" : "Jon’s iPhone (2)",
    ///   "osBuildUpdate" : "22G86",
    ///   "osVersionNumber" : "18.6",
    ///   "rootFileSystemIsWritable" : false
    /// }
    /// ```
    async fn get_ios_device_uuid(device_name_query: &str) -> Result<String> {
        use serde_json::Value;

        let tmpfile = tempfile::NamedTempFile::new()
            .context("Failed to create temporary file for device list")?;

        Command::new("xcrun")
            .args([
                "devicectl".to_string(),
                "list".to_string(),
                "devices".to_string(),
                "--json-output".to_string(),
                tmpfile.path().to_str().unwrap().to_string(),
            ])
            .output()
            .await?;

        let json: Value = serde_json::from_str(&std::fs::read_to_string(tmpfile.path())?)
            .context("Failed to parse xcrun output")?;

        let devices = json
            .get("result")
            .context("Failed to parse xcrun output")?
            .get("devices")
            .context("Failed to parse xcrun output")?
            .as_array()
            .context("Failed to get devices from xcrun output")?;

        // by default, we just pick the first available device and then look for better fits.
        let mut device_idx = 0;

        match device_name_query.is_empty() {
            // If the user provided a query, then we look through the device list looking for the right one.
            // This searches both UUIDs and names, making it possible to paste an ID or a name.
            false => {
                use nucleo::{chars, Config, Matcher, Utf32Str};
                let normalize = |c: char| chars::to_lower_case(chars::normalize(c));
                let mut matcher = Matcher::new(Config::DEFAULT);
                let mut best_score = 0;
                let needle = device_name_query.chars().map(normalize).collect::<String>();
                for (idx, device) in devices.iter().enumerate() {
                    let device_name = device
                        .get("deviceProperties")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or_default();
                    let device_uuid = device
                        .get("identifier")
                        .and_then(|n| n.as_str())
                        .unwrap_or_default();
                    let haystack = format!("{device_name} {device_uuid}")
                        .chars()
                        .map(normalize)
                        .collect::<String>();
                    let name_score = matcher.fuzzy_match(
                        Utf32Str::Ascii(haystack.as_bytes()),
                        Utf32Str::Ascii(needle.as_bytes()),
                    );
                    if let Some(score) = name_score {
                        if score > best_score {
                            best_score = score;
                            device_idx = idx;
                        }
                    }
                }

                if best_score == 0 {
                    tracing::warn!(
                        "No device found matching query: {device_name_query}. Using first available device."
                    );
                }
            }

            // If the query is empty, then we just find the first connected/available device
            // This is somewhat based on the bundle format, since we don't want to accidentally upload
            // iOS apps to watches/tvs
            true => {
                for (idx, device) in devices.iter().enumerate() {
                    let is_paired = device
                        .get("connectionProperties")
                        .and_then(|g| g.get("pairingState"))
                        .map(|s| s.as_str() == Some("paired"))
                        .unwrap_or(false);

                    let is_ios_device = matches!(
                        device.get("deviceType").and_then(|s| s.as_str()),
                        Some("iPhone") | Some("iPad") | Some("iPod")
                    );

                    if is_paired && is_ios_device {
                        device_idx = idx;
                        break;
                    }
                }
            }
        }

        devices
            .get(device_idx)
            .context("No devices found")?
            .get("identifier")
            .and_then(|id| id.as_str())
            .map(|s| s.to_string())
            .context("Failed to extract device UUID")
    }

    async fn get_ios_installation_url(device_uuid: &str, app_path: &Path) -> Result<String> {
        let tmpfile = tempfile::NamedTempFile::new()
            .context("Failed to create temporary file for device list")?;

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
            ])
            .arg(tmpfile.path())
            .output()
            .await?;

        if !output.status.success() {
            bail!(
                "Failed to install app: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(tmpfile.path())?)
                .context("Failed to parse xcrun output")?;
        let installation_url = json["result"]["installedApplications"][0]["installationURL"]
            .as_str()
            .context("Failed to extract installation URL from xcrun output")?
            .to_string();

        Ok(installation_url)
    }

    async fn launch_ios_app_paused(device_uuid: &str, installation_url: &str) -> Result<()> {
        let tmpfile = tempfile::NamedTempFile::new()
            .context("Failed to create temporary file for device list")?;

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
            ])
            .arg(tmpfile.path())
            .output()
            .await?;

        if !output.status.success() {
            bail!("Failed to launch app: {output:?}");
        }

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(tmpfile.path())?)
                .context("Failed to parse xcrun output")?;

        let status_pid = json["result"]["process"]["processIdentifier"]
            .as_u64()
            .context("Failed to extract process identifier")?;

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
            bail!("Failed to resume app: {output:?}");
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
    async fn open_android(
        &mut self,
        root: bool,
        devserver_socket: SocketAddr,
        envs: Vec<(String, String)>,
        device_name_query: Option<String>,
    ) -> Result<()> {
        let apk_path = self.build.debug_apk_path();
        let session_cache = self.build.session_cache_dir();
        let application_id = self.build.bundle_identifier();
        let adb = self.build.workspace.android_tools()?.adb.clone();
        let (stdout_tx, stdout_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        // Start backgrounded since .open() is called while in the arm of the top-level match
        let task = tokio::task::spawn(async move {
            // call `adb root` so we can push patches to the device
            if root {
                if let Err(e) = Command::new(&adb).arg("root").output().await {
                    tracing::error!("Failed to run `adb root`: {e}");
                }
            }

            // Try to get the transport ID for the device in case there are multiple specified devices
            // All future commands should use this since its the most recent.
            let transport_id_args =
                Self::get_android_device_transport_id(&adb, device_name_query.as_deref()).await;

            // Wait for device to be ready
            let cmd = Command::new(&adb)
                .args(transport_id_args)
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

            let port = devserver_socket.port();
            if let Err(e) = Command::new(&adb)
                .arg("reverse")
                .arg(format!("tcp:{port}"))
                .arg(format!("tcp:{port}"))
                .output()
                .await
            {
                tracing::error!("failed to forward port {port}: {e}");
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

            // Try to get the transport ID for the device
            let transport_id_args =
                Self::get_android_device_transport_id(&adb, device_name_query.as_deref()).await;

            // Get the app's PID with retries
            // Retry up to 10 times (10 seconds total) since app launch is asynchronous
            let mut pid: Option<String> = None;
            for attempt in 1..=10 {
                match Self::get_android_app_pid(&adb, &application_id, &transport_id_args).await {
                    Ok(p) => {
                        pid = Some(p);
                        break;
                    }
                    Err(_) if attempt < 10 => {
                        tracing::debug!(
                            "App PID not found yet, retrying in 1 second... (attempt {}/10)",
                            attempt
                        );
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    Err(e) => {
                        return Err(e).context(
                            "Failed to get app PID after 10 attempts - app may not have started",
                        );
                    }
                }
            }

            let pid = pid.context("Failed to get app PID")?;

            // Spawn logcat with filtering
            // By default: show only RustStdoutStderr (app Rust logs) and fatal errors
            // With tracing enabled: show all logs from the app process
            // Note: We always capture at DEBUG level, then filter in Rust based on trace flag
            let mut child = Command::new(&adb)
                .args(&transport_id_args)
                .arg("logcat")
                .arg("-v")
                .arg("brief")
                .arg("--pid")
                .arg(&pid)
                .arg("*:D") // Capture all logs at DEBUG level (filtered in Rust)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()?;

            let stdout = child.stdout.take().unwrap();
            let mut reader = BufReader::new(stdout).lines();

            while let Ok(Some(line)) = reader.next_line().await {
                _ = stdout_tx.send(line);
            }

            Ok::<(), Error>(())
        });

        self.spawn_handle = Some(task);
        self.adb_logcat_stdout = Some(UnboundedReceiverStream::new(stdout_rx));

        Ok(())
    }

    fn make_entropy_path(exe: &PathBuf) -> PathBuf {
        let id = uuid::Uuid::new_v4();
        let name = id.to_string();
        let some_entropy = name.split('-').next().unwrap();

        // Split up the exe into the file stem and extension
        let extension = exe.extension().unwrap_or_default();
        let file_stem = exe.file_stem().unwrap().to_str().unwrap();

        // Make a copy of the server exe with a new name
        let entropy_server_exe = exe
            .with_file_name(format!("{}-{}", file_stem, some_entropy))
            .with_extension(extension);

        std::fs::copy(exe, &entropy_server_exe).unwrap();

        entropy_server_exe
    }

    fn app_exe(&mut self) -> PathBuf {
        let mut main_exe = self.build.main_exe();

        // The requirement here is based on the platform, not necessarily our current architecture.
        let requires_entropy = match self.build.bundle {
            // When running "bundled", we don't need entropy
            BundleFormat::Web | BundleFormat::MacOS | BundleFormat::Ios | BundleFormat::Android => {
                false
            }

            // But on platforms that aren't running as "bundled", we do.
            BundleFormat::Windows | BundleFormat::Linux | BundleFormat::Server => true,
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
        let url = match self.build.bundle {
            BundleFormat::MacOS
            | BundleFormat::Windows
            | BundleFormat::Linux
            | BundleFormat::Server => {
                let Some(Some(pid)) = self.child.as_mut().map(|f| f.id()) else {
                    tracing::warn!("No process to attach debugger to");
                    return Ok(());
                };

                format!(
                    "vscode://vadimcn.vscode-lldb/launch/config?{{'request':'attach','pid':{pid}}}"
                )
            }

            BundleFormat::Web => {
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

            BundleFormat::Ios => {
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
            BundleFormat::Android => {
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

        tracing::info!("Opening debugger for [{}]: {url}", self.build.bundle);

        _ = tokio::process::Command::new("code")
            .arg("--open-url")
            .arg(url)
            .spawn();

        Ok(())
    }

    async fn get_android_device_transport_id(
        adb: &PathBuf,
        device_name_query: Option<&str>,
    ) -> Vec<String> {
        // If there are multiple devices, we pick the one matching the query
        let mut device_specifier_args = vec![];

        if let Some(device_name_query) = device_name_query {
            if let Ok(res) = Command::new(adb).arg("devices").arg("-l").output().await {
                let devices = String::from_utf8_lossy(&res.stdout);
                let mut best_score = 0;
                let mut device_identifier = "".to_string();
                use nucleo::{chars, Config, Matcher, Utf32Str};
                let mut matcher = Matcher::new(Config::DEFAULT);
                let normalize = |c: char| chars::to_lower_case(chars::normalize(c));
                let needle = device_name_query.chars().map(normalize).collect::<Vec<_>>();

                for line in devices.lines() {
                    let device_name = line.split_whitespace().next().unwrap_or("");
                    let Some(transport_id) = line
                        .split_whitespace()
                        .find(|s| s.starts_with("transport_id:"))
                        .map(|s| s.trim_start_matches("transport_id:"))
                    else {
                        continue;
                    };

                    let device_name = device_name.chars().map(normalize).collect::<Vec<_>>();
                    let score = matcher
                        .fuzzy_match(Utf32Str::Unicode(&device_name), Utf32Str::Unicode(&needle));
                    if let Some(score) = score {
                        if score > best_score {
                            best_score = score;
                            device_identifier = transport_id.to_string();
                        }
                    }
                }

                if best_score != 0 {
                    device_specifier_args.push("-t".to_string());
                    device_specifier_args.push(device_identifier.to_string());
                }
            }

            if device_specifier_args.is_empty() {
                tracing::warn!(
                    "No device found matching query: {device_name_query}. Using default transport ID."
                );
            }
        }

        device_specifier_args
    }

    /// Get the PID of the running Android app
    async fn get_android_app_pid(
        adb: &Path,
        application_id: &str,
        transport_id_args: &[String],
    ) -> Result<String> {
        let output = Command::new(adb)
            .args(transport_id_args)
            .arg("shell")
            .arg("pidof")
            .arg(application_id)
            .output()
            .await?;

        let pid = String::from_utf8(output.stdout)?.trim().to_string();

        if pid.is_empty() {
            anyhow::bail!("App process not found - may not have started yet");
        }

        Ok(pid)
    }
}
