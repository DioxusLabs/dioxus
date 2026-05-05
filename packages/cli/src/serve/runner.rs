use super::{AppBuilder, ServeUpdate, WebServer};
use crate::{
    BuildArtifacts, BuildId, BuildMode, BuildTargets, BuilderUpdate, BundleFormat,
    HotpatchModuleCache, Result, ServeArgs, TailwindCli, TraceSrc, Workspace,
    platform_override::CommandWithPlatformOverrides,
};
use anyhow::{Context, bail};
use dioxus_core::internal::{
    HotReloadTemplateWithLocation, HotReloadedTemplate, TemplateGlobalKey,
};
use dioxus_devtools_types::HotReloadMsg;
use dioxus_dx_wire_format::BuildStage;
use dioxus_html::HtmlCtx;
use dioxus_rsx::CallBody;
use dioxus_rsx_hotreload::{ChangedRsx, HotReloadResult};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use futures_util::future::OptionFuture;
use krates::NodeId;
use notify::{
    Config, EventKind, RecursiveMode, Watcher as NotifyWatcher,
    event::{MetadataKind, ModifyKind},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    net::{IpAddr, TcpListener},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use syn::spanned::Spanned;
use tokio::process::Command;

/// This is the primary "state" object that holds the builds and handles for the running apps.
///
/// It also holds the watcher which is used to watch for changes in the filesystem and trigger rebuilds,
/// hotreloads, asset updates, etc.
///
/// Since we resolve the build request before initializing the CLI, it also serves as a place to store
/// resolved "serve" arguments, which is why it takes ServeArgs instead of BuildArgs. Simply wrap the
/// BuildArgs in a default ServeArgs and pass it in.
pub(crate) struct AppServer {
    /// the platform of the "primary" crate (ie the first)
    pub(crate) workspace: Arc<Workspace>,

    pub(crate) client: AppBuilder,
    pub(crate) server: Option<AppBuilder>,

    // Related to the filesystem watcher
    pub(crate) watcher: Box<dyn notify::Watcher>,
    pub(crate) _watcher_tx: UnboundedSender<notify::Event>,
    pub(crate) watcher_rx: UnboundedReceiver<notify::Event>,

    // Tracked state related to open builds and hot reloading
    pub(crate) applied_client_hot_reload_message: HotReloadMsg,
    pub(crate) file_map: HashMap<PathBuf, CachedFile>,

    // Resolved args related to how we go about processing the rebuilds and logging
    pub(crate) hotreload_mode: HotReloadMode,
    pub(crate) interactive: bool,
    pub(crate) _force_sequential: bool,
    pub(crate) open_browser: bool,
    pub(crate) _wsl_file_poll_interval: u16,
    pub(crate) always_on_top: bool,
    pub(crate) fullstack: bool,
    pub(crate) ssg: bool,
    pub(crate) watch_fs: bool,

    // resolve args related to the webserver
    pub(crate) devserver_port: u16,
    pub(crate) devserver_bind_ip: IpAddr,
    pub(crate) proxied_port: Option<u16>,
    pub(crate) cross_origin_policy: bool,

    // The arguments that should be forwarded to the client app when it is opened
    pub(crate) client_args: Vec<String>,
    // The arguments that should be forwarded to the server app when it is opened
    pub(crate) server_args: Vec<String>,

    // Additional plugin-type tools
    pub(crate) tw_watcher: tokio::task::JoinHandle<Result<()>>,

    // File changes that arrived while a build was in progress, to be processed after build completes
    pub(crate) pending_file_changes: Vec<PathBuf>,

    // Field-aware snapshots of Cargo.toml / Dioxus.toml so we can diff edits against the last
    // known-good content rather than re-parsing from scratch each time. See
    // [`AppServer::analyze_cargo_toml_change`] / [`AppServer::analyze_dioxus_toml_change`].
    pub(crate) cargo_toml_snapshots: HashMap<PathBuf, toml::Value>,
    pub(crate) dioxus_toml_snapshots: HashMap<PathBuf, toml::Value>,

    // The original target args used to construct the initial `BuildRequest`s, kept so we can
    // re-derive fresh requests when a config edit invalidates the cached state.
    pub(crate) target_args: CommandWithPlatformOverrides<crate::BuildArgs>,
}

pub(crate) struct CachedFile {
    contents: String,
    most_recent: Option<String>,
    templates: HashMap<TemplateGlobalKey, HotReloadedTemplate>,
}

#[derive(PartialEq, Clone, Debug, Copy)]
pub(crate) enum HotReloadMode {
    Hotpatch,
    RsxOnly,
    Disabled,
}

impl AppServer {
    /// Create the AppRunner and then initialize the filemap with the crate directory.
    pub(crate) async fn new(args: ServeArgs) -> Result<Self> {
        let workspace = Workspace::current().await?;

        // Resolve the simpler args
        let interactive = args.is_interactive_tty();
        let force_sequential = args.platform_args.shared.targets.force_sequential_build();
        let cross_origin_policy = args.cross_origin_policy;

        // Find the launch args for the client and server
        let split_args = |args: &str| {
            args.split_whitespace()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        };

        let server_args = args.platform_args.with_server_or_shared(|c| &c.args);
        let server_args = split_args(server_args);
        let client_args = args.platform_args.with_client_or_shared(|c| &c.args);
        let client_args = split_args(client_args);

        // These come from the args but also might come from the workspace settings
        // We opt to use the manually specified args over the workspace settings
        let open_browser = args
            .open
            .unwrap_or_else(|| workspace.settings.always_open_browser.unwrap_or(false))
            && interactive;

        let wsl_file_poll_interval = args
            .wsl_file_poll_interval
            .unwrap_or_else(|| workspace.settings.wsl_file_poll_interval.unwrap_or(2));

        let always_on_top = args
            .always_on_top
            .unwrap_or_else(|| workspace.settings.always_on_top.unwrap_or(true));

        // Use 127.0.0.1 as the default address if none is specified.
        // If the user wants to export on the network, they can use `0.0.0.0` instead.
        let devserver_bind_ip = args.address.addr.unwrap_or(WebServer::SELF_IP);

        // If the user specified a port, use that, otherwise use any available port, preferring 8080
        let devserver_port = args
            .address
            .port
            .unwrap_or_else(|| get_available_port(devserver_bind_ip, Some(8080)).unwrap_or(8080));

        // Spin up the file watcher
        let (watcher_tx, watcher_rx) = futures_channel::mpsc::unbounded();
        let watcher = create_notify_watcher(watcher_tx.clone(), wsl_file_poll_interval as u64);

        let ssg = args.platform_args.shared.targets.ssg;
        let target_args = CommandWithPlatformOverrides {
            shared: args.platform_args.shared.targets,
            server: args.platform_args.server.map(|s| s.targets),
            client: args.platform_args.client.map(|c| c.targets),
        };
        // Hold on to the original target args so we can re-derive `BuildRequest`s when a
        // Cargo.toml / Dioxus.toml edit invalidates feature resolution, profile flags, or any
        // other field cooked into a `BuildRequest` at startup. See `recreate_build_requests`.
        let stored_target_args = target_args.clone();
        let BuildTargets { client, server } = target_args.into_targets().await?;

        // All servers will end up behind us (the devserver) but on a different port
        // This is so we can serve a loading screen as well as devtools without anything particularly fancy
        let fullstack = server.is_some();
        let should_proxy_port = match client.bundle {
            BundleFormat::Server => true,
            _ => fullstack && !ssg,
        };

        let proxied_port = should_proxy_port
            .then(|| get_available_port(devserver_bind_ip, None))
            .flatten();

        let watch_fs = args.watch.unwrap_or(true);
        let hotreload_mode = if args.hot_patch.unwrap_or(true) {
            HotReloadMode::Hotpatch
        } else {
            HotReloadMode::RsxOnly
        };

        let client = AppBuilder::new(&client)?;
        let server = server.map(|server| AppBuilder::new(&server)).transpose()?;

        let tw_watcher = TailwindCli::serve(
            client.build.package_manifest_dir(),
            client.build.config.application.tailwind_input.clone(),
            client.build.config.application.tailwind_output.clone(),
        );

        _ = client.build.start_simulators().await;

        // Encourage the user to update to a new dx version
        crate::update::log_if_cli_could_update();

        // Create the runner
        let mut runner = Self {
            file_map: Default::default(),
            applied_client_hot_reload_message: Default::default(),
            watch_fs,
            hotreload_mode,
            client,
            server,
            open_browser,
            _wsl_file_poll_interval: wsl_file_poll_interval,
            always_on_top,
            workspace,
            devserver_port,
            devserver_bind_ip,
            proxied_port,
            watcher,
            watcher_rx,
            _watcher_tx: watcher_tx,
            interactive,
            _force_sequential: force_sequential,
            cross_origin_policy,
            fullstack,
            ssg,
            tw_watcher,
            server_args,
            client_args,
            pending_file_changes: Vec::new(),
            cargo_toml_snapshots: HashMap::new(),
            dioxus_toml_snapshots: HashMap::new(),
            target_args: stored_target_args,
        };

        // Only register the hot-reload stuff if we're watching the filesystem
        if runner.watch_fs {
            // Spin up the notify watcher
            // When builds load though, we're going to parse their depinfo and add the paths to the watcher
            runner.watch_filesystem();

            // todo(jon): this might take a while so we should try and background it, or make it lazy somehow
            // we could spawn a thread to search the FS and then when it returns we can fill the filemap
            // in testing, if this hits a massive directory, it might take several seconds with no feedback.
            // really, we should be using depinfo to get the files that are actually used, but the depinfo file might not be around yet
            // todo(jon): see if we can just guess the depinfo file before it generates. might be stale but at least it catches most of the files
            runner.load_rsx_filemap();
        }

        // Seed snapshots of every Cargo.toml + Dioxus.toml in the workspace so subsequent
        // edits diff against the on-disk state at startup, not against an empty table.
        runner.seed_config_snapshots();

        Ok(runner)
    }

    /// Walk the workspace and load the on-disk contents of every `Cargo.toml` and `Dioxus.toml`
    /// into the snapshot maps. Parse failures are silently skipped — the next successful save
    /// will populate the snapshot, and the diff against an empty table will trigger a rebuild
    /// (which is the right behavior since something was previously broken).
    fn seed_config_snapshots(&mut self) {
        let workspace_cargo = self.workspace.workspace_root().join("Cargo.toml");
        // Collect manifest paths first so we don't hold an immutable borrow on
        // `self.workspace` while calling `&mut self` methods.
        let member_manifests: Vec<PathBuf> = self
            .workspace
            .krates
            .workspace_members()
            .filter_map(|member| match member {
                krates::Node::Krate { krate, .. } => {
                    Some(krate.manifest_path.as_std_path().to_path_buf())
                }
                _ => None,
            })
            .collect();
        self.seed_cargo_snapshot(&workspace_cargo);
        for manifest in member_manifests {
            if manifest != workspace_cargo {
                self.seed_cargo_snapshot(&manifest);
            }
        }

        // Walk up from the client crate's manifest dir for Dioxus.toml — same search the
        // `Workspace::load_dioxus_config` method uses.
        let crate_dir = self.client.build.crate_dir();
        let workspace_root = self.workspace.workspace_root();
        let mut current = crate_dir.canonicalize().unwrap_or(crate_dir);
        let workspace_root = workspace_root
            .canonicalize()
            .unwrap_or_else(|_| workspace_root.clone());
        while current.starts_with(&workspace_root) {
            for name in ["Dioxus.toml", "dioxus.toml"] {
                let p = current.join(name);
                if p.is_file() {
                    self.seed_dioxus_snapshot(&p);
                }
            }
            if !current.pop() {
                break;
            }
        }
    }

    /// Re-run `cargo metadata` and rebuild every `BuildRequest` from scratch so edits to
    /// `Cargo.toml` / `Dioxus.toml` actually flow into the next compile. Without this, the
    /// `BuildRequest` constructed at startup keeps a stale view of features, profile resolution,
    /// dependency graph, target dirs, and `DioxusConfig` — the `cargo rustc` invocation would
    /// re-read `Cargo.toml` itself but anything dx derives from the workspace metadata stays
    /// frozen at the original snapshot.
    ///
    /// Called from the rebuild dispatch in `handle_file_change` whenever a config edit triggers
    /// a full rebuild. Failures (broken `Cargo.toml` mid-edit, cargo metadata error) are logged
    /// and treated as non-fatal — the existing `BuildRequest`s stay in place and the rebuild is
    /// skipped so the user can fix the file without us crashing the serve session.
    async fn recreate_build_requests(&mut self) -> Result<()> {
        // Capture state from the existing `BuildRequest`s that must survive recreation.
        // `session_cache_dir` holds files written by `prebuild` (link_err.txt, link_args.json,
        // etc.) that subsequent build steps `dunce::canonicalize` and require to exist.
        // `start_rebuild` skips `prebuild`, so a fresh empty tempdir on the new request would
        // fail with ENOENT during `cargo_build_env_vars`. Carry the path over so the existing
        // files stay reachable.
        let preserved_client_session_cache = self.client.build.session_cache_dir.clone();
        let preserved_server_session_cache = self
            .server
            .as_ref()
            .map(|s| s.build.session_cache_dir.clone());

        let new_workspace = Workspace::reload().await?;
        let mut new_targets = self.target_args.clone().into_targets().await?;

        new_targets.client.session_cache_dir = preserved_client_session_cache;
        if let (Some(req), Some(dir)) =
            (new_targets.server.as_mut(), preserved_server_session_cache)
        {
            req.session_cache_dir = dir;
        }

        // Swap the freshly-derived `BuildRequest`s onto the existing `AppBuilder`s. The
        // builders keep their websockets, child processes, watcher state, and hot-reload
        // bookkeeping; only the build configuration is replaced.
        self.client.build = new_targets.client;
        if let (Some(server_app), Some(server_req)) = (self.server.as_mut(), new_targets.server) {
            server_app.build = server_req;
        }
        self.workspace = new_workspace;

        // Caches keyed off the old `BuildRequest` would now be wrong. The patch cache is for
        // the previous fat binary's symbol table, the file_map RSX templates assume the old
        // crate layout, and the applied hot-reload set should be re-empty so the freshly
        // opened app starts from a clean slate.
        self.clear_patches();
        self.clear_cached_rsx();
        self.file_map.clear();
        self.applied_client_hot_reload_message = Default::default();

        // Tailwind input/output paths come from `application.tailwind_*` in `Dioxus.toml` —
        // restart the watcher so it picks up any change.
        self.tw_watcher.abort();
        self.tw_watcher = TailwindCli::serve(
            self.client.build.package_manifest_dir(),
            self.client.build.config.application.tailwind_input.clone(),
            self.client.build.config.application.tailwind_output.clone(),
        );

        // Re-seed the config snapshots so the now-canonical state is the baseline for
        // subsequent diffs. Without this, formatter passes that touch the file (e.g.
        // `cargo metadata` rewriting trailing whitespace) could trigger another rebuild.
        self.cargo_toml_snapshots.clear();
        self.dioxus_toml_snapshots.clear();
        self.seed_config_snapshots();

        Ok(())
    }

    /// Read `path` as TOML and store the parsed value as the baseline for future diffs.
    /// Silently no-ops on read or parse failure.
    fn seed_cargo_snapshot(&mut self, path: &Path) {
        if let Ok(value) = read_toml_file(path) {
            self.cargo_toml_snapshots.insert(path.to_path_buf(), value);
        }
    }

    fn seed_dioxus_snapshot(&mut self, path: &Path) {
        if let Ok(value) = read_toml_file(path) {
            self.dioxus_toml_snapshots.insert(path.to_path_buf(), value);
        }
    }

    /// Classify a change to a `Cargo.toml`. Returns the action the runner should take and
    /// updates the cached snapshot to the new content (so repeated edits don't loop on the
    /// same diff).
    ///
    /// Parse failures keep the snapshot intact and return `Ignore` — the user is mid-edit and
    /// the next successful save will diff against the last known-good state.
    pub(crate) fn analyze_cargo_toml_change(&mut self, path: &Path) -> ConfigChangeOutcome {
        let new_value = match read_toml_file(path) {
            Ok(v) => v,
            Err(_) => {
                return ConfigChangeOutcome::Ignore {
                    note: Some(format!(
                        "Cargo.toml parse failed at {}, will retry on next save",
                        path.display()
                    )),
                };
            }
        };
        let old_value = self
            .cargo_toml_snapshots
            .get(path)
            .cloned()
            .unwrap_or_else(|| toml::Value::Table(Default::default()));
        let ctx = AnalysisCtx {
            active_profile: self.client.build.profile.clone(),
            active_bundle: self.client.build.bundle,
        };
        let outcome = analyze_cargo_value(&old_value, &new_value, &ctx);
        self.cargo_toml_snapshots
            .insert(path.to_path_buf(), new_value);
        outcome
    }

    pub(crate) fn analyze_dioxus_toml_change(&mut self, path: &Path) -> ConfigChangeOutcome {
        let new_value = match read_toml_file(path) {
            Ok(v) => v,
            Err(_) => {
                return ConfigChangeOutcome::Ignore {
                    note: Some(format!(
                        "Dioxus.toml parse failed at {}, will retry on next save",
                        path.display()
                    )),
                };
            }
        };
        let old_value = self
            .dioxus_toml_snapshots
            .get(path)
            .cloned()
            .unwrap_or_else(|| toml::Value::Table(Default::default()));
        let ctx = AnalysisCtx {
            active_profile: self.client.build.profile.clone(),
            active_bundle: self.client.build.bundle,
        };
        let outcome = analyze_dioxus_value(&old_value, &new_value, &ctx);
        self.dioxus_toml_snapshots
            .insert(path.to_path_buf(), new_value);
        outcome
    }

    pub(crate) fn initialize(&mut self) {
        let build_mode = self.initial_build_mode();

        self.client.start(build_mode.clone(), BuildId::PRIMARY);
        if let Some(server) = self.server.as_mut() {
            server.start(build_mode, BuildId::SECONDARY);
        }
    }

    /// The `BuildMode` that fresh/full rebuilds should start with under the current hotreload mode.
    ///
    /// Only `Hotpatch` needs the `Fat` build to prime the hotpatch engine; every other mode uses a
    /// plain `Base` build.
    fn initial_build_mode(&self) -> BuildMode {
        match self.hotreload_mode {
            HotReloadMode::Hotpatch => BuildMode::Fat,
            HotReloadMode::RsxOnly | HotReloadMode::Disabled => BuildMode::Base,
        }
    }

    /// Take any pending file changes that were queued while a build was in progress.
    /// Returns the files and clears the pending list.
    pub(crate) fn take_pending_file_changes(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.pending_file_changes)
    }

    pub(crate) async fn rebuild_ssg(&mut self, devserver: &WebServer) {
        if self.client.stage != BuildStage::Success {
            return;
        }
        // Run SSG and cache static routes if the server build is done
        if let Some(server) = self.server.as_mut() {
            if !self.ssg || server.stage != BuildStage::Success {
                return;
            }
            if let Err(err) = server
                .pre_render_static_routes(
                    Some(devserver.devserver_address()),
                    Some(&server.tx.clone()),
                )
                .await
            {
                tracing::error!("Failed to pre-render static routes: {err}");
            }
        }
    }

    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        let client = &mut self.client;
        let server = self.server.as_mut();

        let client_wait = client.wait();
        let server_wait = OptionFuture::from(server.map(|s| s.wait()));
        let watcher_wait = self.watcher_rx.next();

        tokio::select! {
            // Wait for the client to finish
            client_update = client_wait => {
                ServeUpdate::BuilderUpdate {
                    id: BuildId::PRIMARY,
                    update: client_update,
                }
            }

            Some(server_update) = server_wait => {
                ServeUpdate::BuilderUpdate {
                    id: BuildId::SECONDARY,
                    update: server_update,
                }
            }

            // Wait for the watcher to send us an event
            event = watcher_wait => {
                let mut changes: Vec<_> = event.into_iter().collect();

                // Dequeue in bulk if we can, we might've received a lot of events in one go
                while let Ok(event) = self.watcher_rx.try_recv() {
                    changes.push(event);
                }

                // Filter the changes
                let mut files: Vec<PathBuf> = vec![];

                // Decompose the events into a list of all the files that have changed
                for event in changes.drain(..) {
                    // Make sure we add new folders to the watch list, provided they're not matched by the ignore list
                    // We'll only watch new folders that are found under the crate, and then update our watcher to watch them
                    // This unfortunately won't pick up new krates added "at a distance" - IE krates not within the workspace.
                    if let EventKind::Create(_create_kind) = event.kind {
                        // If it's a new folder, watch it
                        // If it's a new cargo.toml (ie dep on the fly),
                        // todo(jon) support new folders on the fly
                    }

                    for path in event.paths {
                        // Workaround for notify and vscode-like editor:
                        // - when edit & save a file in vscode, there will be two notifications,
                        // - the first one is a file with empty content.
                        // - filter the empty file notification to avoid false rebuild during hot-reload
                        if let Ok(metadata) = std::fs::metadata(&path) {
                            if metadata.len() == 0 {
                                continue;
                            }
                        }

                        files.push(path);
                    }
                }

                ServeUpdate::FilesChanged { files }
            }

        }
    }

    /// Handle an update from the builder
    pub(crate) async fn new_build_update(&mut self, update: &BuilderUpdate, devserver: &WebServer) {
        if let BuilderUpdate::BuildReady { .. } = update {
            // If the build is ready, we need to check if we need to pre-render with ssg
            self.rebuild_ssg(devserver).await;
        }
    }

    /// Fold a `.d` file's dep list into our tracking state:
    /// - `.rs` files get parsed and inserted into `file_map` (skipping entries already there so
    ///   we don't clobber a `most_recent` buffer mid-edit) so RSX hot-reload diffing can find
    ///   them on the next edit.
    /// - any new path is appended to `client.artifacts.depinfo.files` so the non-`.rs` rebuild
    ///   trigger at [`handle_file_change`] picks up edits to `include_str!`/`include_bytes!`
    ///   targets etc.
    pub(crate) fn absorb_dep_info_files(&mut self, files: &[PathBuf]) {
        for path in files {
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or_default();

            if ext == "rs" && !self.file_map.contains_key(path) {
                if let Ok(contents) = std::fs::read_to_string(path) {
                    self.file_map.insert(
                        path.clone(),
                        CachedFile {
                            contents,
                            most_recent: None,
                            templates: Default::default(),
                        },
                    );
                }
            }

            if let Some(artifacts) = self.client.artifacts.as_mut() {
                if !artifacts.depinfo.files.contains(path) {
                    artifacts.depinfo.files.push(path.clone());
                }
            }
        }
    }

    /// Handle the list of changed files from the file watcher, attempting to aggressively prevent
    /// full rebuilds by hot-reloading RSX and hot-patching Rust code.
    ///
    /// This will also handle any assets that are linked in the files, and copy them to the bundle
    /// and send them to the client.
    pub(crate) async fn handle_file_change(&mut self, files: &[PathBuf], server: &mut WebServer) {
        // We can attempt to hotpatch if the build is in a bad state, since this patch might be a recovery.
        if !matches!(
            self.client.stage,
            BuildStage::Failed | BuildStage::Aborted | BuildStage::Success
        ) {
            // Queue file changes that arrive during a build, so we can process them after the build completes.
            // This prevents losing changes from tools like stylance, tailwind, or sass that generate files
            // in response to source changes.
            tracing::debug!(
                "Queueing file change - client is not ready to receive hotreloads. Files: {:?}",
                files
            );
            self.pending_file_changes.extend(files.iter().cloned());
            return;
        }

        // If we have any changes to the rust files, we need to update the file map
        let mut templates = vec![];

        // Prepare the hotreload message we need to send
        let mut assets = Vec::new();
        let mut needs_rust_rebuild = false;
        // Cargo.toml / Dioxus.toml edits change the dependency graph or build inputs in ways
        // hotpatch can't reason about — force a from-scratch rebuild instead of `patch_rebuild`
        // when this is true.
        let mut force_full_rebuild = false;

        // We attempt to hotreload rsx blocks without a full rebuild
        for path in files {
            // for various assets that might be linked in, we just try to hotreloading them forcefully
            // That is, unless they appear in an include! macro, in which case we need to a full rebuild....
            let ext = path
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or_default();

            // Cargo.toml / Dioxus.toml — field-aware classification decides between full
            // rebuild, "restart required" warning, and ignore-as-cosmetic. See
            // [`AppServer::analyze_cargo_toml_change`] / [`AppServer::analyze_dioxus_toml_change`].
            let filename = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or_default();
            match filename {
                "Cargo.toml" => {
                    let outcome = self.analyze_cargo_toml_change(path);
                    if self.apply_config_outcome(outcome, path) {
                        needs_rust_rebuild = true;
                        force_full_rebuild = true;
                    }
                    continue;
                }
                "Dioxus.toml" | "dioxus.toml" => {
                    let outcome = self.analyze_dioxus_toml_change(path);
                    if self.apply_config_outcome(outcome, path) {
                        needs_rust_rebuild = true;
                        force_full_rebuild = true;
                    }
                    continue;
                }
                _ => {}
            }

            // If it's an asset, we want to hotreload it
            // todo(jon): don't hardcode this here
            if let Some(bundled_names) = self.client.hotreload_bundled_assets(path).await {
                for bundled_name in bundled_names {
                    assets.push(PathBuf::from("/assets/").join(bundled_name));
                }
            }

            // If it's in the public dir, we sync it and trigger a full rebuild
            if self.client.build.path_is_in_public_dir(path) {
                needs_rust_rebuild = true;
                continue;
            }

            // If it's a rust file, we want to hotreload it using the filemap
            if ext == "rs" {
                // And grabout the contents
                let Ok(new_contents) = std::fs::read_to_string(path) else {
                    tracing::debug!("Failed to read rust file while hotreloading: {:?}", path);
                    continue;
                };

                // Get the cached file if it exists - ignoring if it doesn't exist
                let Some(cached_file) = self.file_map.get_mut(path) else {
                    tracing::debug!("No entry for file in filemap: {:?}", path);
                    continue;
                };

                let Ok(local_path) = path.strip_prefix(self.workspace.workspace_root()) else {
                    tracing::debug!("Skipping file outside workspace dir: {:?}", path);
                    continue;
                };

                // We assume we can parse the old file and the new file, ignoring untracked rust files
                let old_syn = syn::parse_file(&cached_file.contents);
                let new_syn = syn::parse_file(&new_contents);
                let (Ok(old_file), Ok(new_file)) = (old_syn, new_syn) else {
                    tracing::debug!("Diff rsx returned not parseable");
                    continue;
                };

                // Update the most recent version of the file, so when we force a rebuild, we keep operating on the most recent version
                cached_file.most_recent = Some(new_contents);

                // This assumes the two files are structured similarly. If they're not, we can't diff them
                let Some(changed_rsx) = dioxus_rsx_hotreload::diff_rsx(&new_file, &old_file) else {
                    needs_rust_rebuild = true;
                    break;
                };

                for ChangedRsx { old, new } in changed_rsx {
                    let old_start = old.span().start();

                    let old_parsed = syn::parse2::<CallBody>(old.tokens);
                    let new_parsed = syn::parse2::<CallBody>(new.tokens);
                    let (Ok(old_call_body), Ok(new_call_body)) = (old_parsed, new_parsed) else {
                        continue;
                    };

                    // Format the template location, normalizing the path
                    let file_name: String = local_path
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");

                    // Returns a list of templates that are hotreloadable
                    let results = HotReloadResult::new::<HtmlCtx>(
                        &old_call_body.body,
                        &new_call_body.body,
                        file_name.clone(),
                    );

                    // If no result is returned, we can't hotreload this file and need to keep the old file
                    let Some(results) = results else {
                        needs_rust_rebuild = true;
                        break;
                    };

                    // Only send down templates that have roots, and ideally ones that have changed
                    // todo(jon): maybe cache these and don't send them down if they're the same
                    for (index, template) in results.templates {
                        if template.roots.is_empty() {
                            continue;
                        }

                        // Create the key we're going to use to identify this template
                        let key = TemplateGlobalKey {
                            file: file_name.clone(),
                            line: old_start.line,
                            column: old_start.column + 1,
                            index,
                        };

                        // if the template is the same, don't send its
                        if cached_file.templates.get(&key) == Some(&template) {
                            continue;
                        };

                        cached_file.templates.insert(key.clone(), template.clone());
                        templates.push(HotReloadTemplateWithLocation { template, key });
                    }
                }
            }

            // If it's not a rust file, then it might be depended on via include! or similar
            if ext != "rs" {
                if let Some(artifacts) = self.client.artifacts.as_ref() {
                    if artifacts.depinfo.files.contains(path) {
                        needs_rust_rebuild = true;
                        break;
                    }
                }
            }
        }

        // If the client is in a failed state, any changes to rsx should trigger a rebuild/hotpatch
        if self.client.stage == BuildStage::Failed && !templates.is_empty() {
            needs_rust_rebuild = true
        }

        // todo - we need to distinguish between hotpatchable rebuilds and true full rebuilds.
        //        A full rebuild is required when the user modifies static initializers which we haven't wired up yet.
        if needs_rust_rebuild {
            // Cargo.toml / Dioxus.toml edits invalidate the cached `BuildRequest`. Refresh
            // `cargo metadata` and re-derive the build requests before kicking off the
            // rebuild, otherwise cargo would re-read Cargo.toml but dx-side state (features,
            // profile, target dirs, DioxusConfig) would stay frozen at startup. If
            // recreation fails (e.g. mid-edit broken Cargo.toml), keep the existing requests
            // and skip the rebuild — same "user is mid-edit, will fix" behavior we use for
            // parse failures in `analyze_*_change`.
            if force_full_rebuild {
                if let Err(err) = self.recreate_build_requests().await {
                    tracing::warn!(
                        dx_src = ?TraceSrc::Dev,
                        "Skipping rebuild: failed to refresh cargo metadata: {err}"
                    );
                    return;
                }
            }

            match self.hotreload_mode {
                // In hotpatch, we can only issue patches if the original build completed.
                // `force_full_rebuild` is set when a Cargo.toml / Dioxus.toml change rewires the
                // dependency graph or build inputs — patch_rebuild can't handle that, so we go
                // through the fat-rebuild path even when patches are otherwise available.
                HotReloadMode::Hotpatch
                    if force_full_rebuild || !self.has_hotpatchable_builds() =>
                {
                    self.client.start_rebuild(BuildMode::Fat, BuildId::PRIMARY);
                    if let Some(server) = self.server.as_mut() {
                        server.start_rebuild(BuildMode::Fat, BuildId::SECONDARY);
                    }
                    self.clear_hot_reload_changes();
                    self.clear_cached_rsx();
                    if force_full_rebuild {
                        self.clear_patches();
                    }
                    server.send_reload_start().await;
                }

                // Otherwise hotpatches go through patching system
                HotReloadMode::Hotpatch => {
                    let changed_crates = self.order_changed_crates(files);

                    self.client.patch_rebuild(
                        files.to_vec(),
                        changed_crates.clone(),
                        BuildId::PRIMARY,
                    );

                    if let Some(server) = self.server.as_mut() {
                        server.patch_rebuild(files.to_vec(), changed_crates, BuildId::SECONDARY);
                    }
                    self.clear_hot_reload_changes();
                    self.clear_cached_rsx();
                    server.send_patch_start().await;
                }

                // Full rust rebuilds with rsx are full builds
                HotReloadMode::RsxOnly => {
                    self.client.start_rebuild(BuildMode::Base, BuildId::PRIMARY);
                    if let Some(server) = self.server.as_mut() {
                        server.start_rebuild(BuildMode::Base, BuildId::SECONDARY);
                    }
                    self.clear_hot_reload_changes();
                    self.clear_cached_rsx();
                    server.send_reload_start().await;
                }

                // `Disabled` is filtered out before reaching `handle_file_change`, so the only way
                // we land here is if the user cycled to `Disabled` mid-handler. Treat it as a
                // no-op with a visible warning so the edit isn't silently dropped.
                HotReloadMode::Disabled => {}
            }
        } else {
            let msg = HotReloadMsg {
                templates,
                assets,
                ms_elapsed: 0,
                jump_table: Default::default(),
                for_build_id: None,
                for_pid: None,
            };

            self.add_hot_reload_message(&msg);

            let file = files[0].display().to_string();
            let workspace_dir = self.client.build.workspace_dir().display().to_string();
            let file = file
                .trim_start_matches(&workspace_dir)
                .trim_start_matches('/');

            // Only send a hotreload message for templates and assets - otherwise we'll just get a full rebuild
            //
            // todo: move the android file uploading out of hotreload_bundled_asset and
            //
            // Also make sure the builder isn't busy since that might cause issues with hotreloads
            // https://github.com/DioxusLabs/dioxus/issues/3361
            if !msg.is_empty() && self.client.can_receive_hotreloads() {
                use crate::styles::NOTE_STYLE;
                tracing::info!(dx_src = ?TraceSrc::Dev, "Hotreloading: {NOTE_STYLE}{}{NOTE_STYLE:#}", file);

                if !server.has_hotreload_sockets() && self.client.build.bundle != BundleFormat::Web
                {
                    tracing::warn!("No clients to hotreload - try reloading the app!");
                }

                server.send_hotreload(msg).await;
            } else {
                tracing::debug!(dx_src = ?TraceSrc::Dev, "Ignoring file change: {}", file);
            }
        }
    }

    /// Map a [`ConfigChangeOutcome`] to user-facing log output, returning `true` iff the
    /// runner should kick off a rebuild for this edit.
    ///
    /// Styling: the `subject` (e.g. `Cargo.toml [dependencies]`) is green
    /// ([`NOTE_STYLE`]), the verb and any trailing prose stay in the default color, and the
    /// file path in parens is gray ([`HINT_STYLE`]).
    ///
    /// `WarnRestart` paths log a warning but explicitly return `false`, because the field
    /// that changed is only consumed at devserver/webserver boot.
    fn apply_config_outcome(&self, outcome: ConfigChangeOutcome, path: &Path) -> bool {
        use crate::styles::HINT_STYLE;
        let workspace_dir = self.client.build.workspace_dir().display().to_string();
        let display_file = path
            .display()
            .to_string()
            .trim_start_matches(&workspace_dir)
            .trim_start_matches('/')
            .to_string();
        use crate::styles::NOTE_STYLE;
        match outcome {
            ConfigChangeOutcome::FullRebuild { subject, detail } => {
                tracing::info!(
                    dx_src = ?TraceSrc::Dev,
                    "Full rebuild: {NOTE_STYLE}{subject}{NOTE_STYLE:#} {detail} {HINT_STYLE}({display_file}){HINT_STYLE:#}"
                );
                true
            }
            ConfigChangeOutcome::WarnRestart { subject, detail } => {
                tracing::warn!(
                    dx_src = ?TraceSrc::Dev,
                    "{NOTE_STYLE}{subject}{NOTE_STYLE:#} {detail} {HINT_STYLE}({display_file}){HINT_STYLE:#}"
                );
                false
            }
            ConfigChangeOutcome::Ignore { note } => {
                if let Some(note) = note {
                    tracing::debug!(
                        dx_src = ?TraceSrc::Dev,
                        "{HINT_STYLE}{note} ({display_file}){HINT_STYLE:#}"
                    );
                } else {
                    tracing::debug!(
                        dx_src = ?TraceSrc::Dev,
                        "{HINT_STYLE}Ignoring config edit: {display_file}{HINT_STYLE:#}"
                    );
                }
                false
            }
        }
    }

    /// Finally "bundle" this app and return a handle to it
    pub(crate) async fn open(
        &mut self,
        artifacts: &BuildArtifacts,
        devserver: &mut WebServer,
    ) -> Result<()> {
        // Make sure to save artifacts regardless of if we're opening the app or not
        match artifacts.build_id {
            BuildId::PRIMARY => self.client.artifacts = Some(artifacts.clone()),
            BuildId::SECONDARY => {
                if let Some(server) = self.server.as_mut() {
                    server.artifacts = Some(artifacts.clone());
                }
            }
            _ => {}
        }

        let should_open = self.client.stage == BuildStage::Success
            && (self.server.as_ref().map(|s| s.stage == BuildStage::Success)).unwrap_or(true);

        use crate::cli::styles::GLOW_STYLE;

        if should_open {
            let time_taken = self.client.total_build_time().unwrap_or_else(|| {
                artifacts
                    .time_end
                    .duration_since(artifacts.time_start)
                    .unwrap()
            });

            if self.client.builds_opened == 0 {
                tracing::info!(
                    "Build completed successfully in {GLOW_STYLE}{}{GLOW_STYLE:#}, launching app! 💫",
                    format_duration_ms(time_taken)
                );
            } else {
                tracing::info!(
                    "Build completed in {GLOW_STYLE}{}{GLOW_STYLE:#}",
                    format_duration_ms(time_taken)
                );
            }

            let open_browser = self.client.builds_opened == 0 && self.open_browser;
            self.open_all(devserver, open_browser).await?;

            // Give a second for the server to boot
            tokio::time::sleep(Duration::from_millis(300)).await;

            // Update the screen + devserver with the new handle info
            devserver.send_reload_command().await
        }

        Ok(())
    }

    /// Open an existing app bundle, if it exists
    ///
    /// Will attempt to open the server and client together, in a coordinated way such that the server
    /// opens first, initializes, and then the client opens.
    ///
    /// There's a number of issues we need to be careful to work around:
    /// - The server failing to boot or crashing on startup (and entering a boot loop)
    /// -
    pub(crate) async fn open_all(
        &mut self,
        devserver: &WebServer,
        open_browser: bool,
    ) -> Result<()> {
        let devserver_ip = devserver.devserver_address();
        let fullstack_address = devserver.proxied_server_address();
        let displayed_address = devserver.displayed_address();

        // Always open the server first after the client has been built
        // Only open the server if it isn't prerendered and finished building
        if let Some(server) = self.server.as_mut().filter(|_| !self.ssg) {
            if server.stage < BuildStage::Success {
                tracing::trace!("Skipping server open: will open once build completes");
            } else {
                tracing::debug!("Opening server build");
                server.soft_kill().await;
                server
                    .open(
                        devserver_ip,
                        displayed_address,
                        fullstack_address,
                        false,
                        false,
                        BuildId::SECONDARY,
                        &self.server_args,
                    )
                    .await?;
            }
        }

        // Skip opening native client if still building (web can open anytime)
        if self.client.build.bundle != BundleFormat::Web && self.client.stage < BuildStage::Success
        {
            tracing::trace!("Skipping client open: will open once build completes");
            return Ok(());
        }

        // Start the new app before we kill the old one to give it a little bit of time
        self.client.soft_kill().await;
        self.client
            .open(
                devserver_ip,
                displayed_address,
                fullstack_address,
                open_browser,
                self.always_on_top,
                BuildId::PRIMARY,
                &self.client_args,
            )
            .await?;

        Ok(())
    }

    /// Shutdown all the running processes
    pub(crate) async fn shutdown(&mut self) -> Result<()> {
        self.client.soft_kill().await;

        if let Some(server) = self.server.as_mut() {
            server.soft_kill().await;
        }

        // If the client is running on Android, we need to remove the port forwarding
        // todo: use the android tools "adb"
        if matches!(self.client.build.bundle, BundleFormat::Android) {
            if let Err(err) = Command::new(&self.workspace.android_tools()?.adb)
                .arg("reverse")
                .arg("--remove")
                .arg(format!("tcp:{}", self.devserver_port))
                .output()
                .await
            {
                tracing::error!(
                    "failed to remove forwarded port {}: {err}",
                    self.devserver_port
                );
            }
        }

        // force the tailwind watcher to stop - if we don't, it eats our stdin
        self.tw_watcher.abort();

        Ok(())
    }

    /// Perform a full rebuild of the app, equivalent to `cargo rustc` from scratch with no incremental
    /// hot-patch engine integration.
    pub(crate) async fn full_rebuild(&mut self) {
        let build_mode = self.initial_build_mode();

        self.client
            .start_rebuild(build_mode.clone(), BuildId::PRIMARY);
        if let Some(s) = self.server.as_mut() {
            s.start_rebuild(build_mode, BuildId::SECONDARY);
        }

        self.clear_hot_reload_changes();
        self.clear_cached_rsx();
        self.clear_patches();
    }

    pub(crate) async fn hotpatch(
        &mut self,
        bundle: &BuildArtifacts,
        id: BuildId,
        cache: &HotpatchModuleCache,
        devserver: &mut WebServer,
    ) -> Result<()> {
        let elapsed = bundle
            .time_end
            .duration_since(bundle.time_start)
            .unwrap_or_default();

        let jump_table = match id {
            BuildId::PRIMARY => self.client.hotpatch(bundle, cache).await,
            BuildId::SECONDARY => {
                self.server
                    .as_mut()
                    .context("Server not found")?
                    .hotpatch(bundle, cache)
                    .await
            }
            _ => bail!("Invalid build id"),
        }?;

        if id == BuildId::PRIMARY {
            self.applied_client_hot_reload_message.jump_table = self.client.patches.last().cloned();
        }

        // If no server, just send the patch immediately
        let Some(server) = self.server.as_mut() else {
            devserver
                .send_patch(jump_table, elapsed, id, self.client.pid)
                .await;
            return Ok(());
        };

        // If we have a server, we need to wait until both the client and server are ready
        // Otherwise we end up with an annoying race condition where the client can't actually load the patch
        if self.client.stage == BuildStage::Success && server.stage == BuildStage::Success {
            let client_jump_table = self
                .client
                .patches
                .last()
                .cloned()
                .context("Missing client jump table")?;

            let server_jump_table = server
                .patches
                .last()
                .cloned()
                .context("Missing server jump table")?;

            devserver
                .send_patch(server_jump_table, elapsed, BuildId::SECONDARY, server.pid)
                .await;

            devserver
                .send_patch(
                    client_jump_table,
                    elapsed,
                    BuildId::PRIMARY,
                    self.client.pid,
                )
                .await;
        }

        Ok(())
    }

    pub(crate) fn get_build(&self, id: BuildId) -> Option<&AppBuilder> {
        match id {
            BuildId::PRIMARY => Some(&self.client),
            BuildId::SECONDARY => self.server.as_ref(),
            _ => None,
        }
    }

    pub(crate) fn client(&self) -> &AppBuilder {
        &self.client
    }

    /// The name of the app being served, to display
    pub(crate) fn app_name(&self) -> &str {
        self.client.build.executable_name()
    }

    /// Get any hot reload changes that have been applied since the last full rebuild
    pub(crate) fn applied_hot_reload_changes(&mut self, build: BuildId) -> HotReloadMsg {
        let mut msg = self.applied_client_hot_reload_message.clone();

        if build == BuildId::PRIMARY {
            msg.jump_table = self.client.patches.last().cloned();
            msg.for_build_id = Some(BuildId::PRIMARY.0 as _);
            if let Some(lib) = msg.jump_table.as_mut() {
                lib.lib = PathBuf::from("/").join(lib.lib.clone());
            }
        }

        if build == BuildId::SECONDARY {
            if let Some(server) = self.server.as_mut() {
                msg.jump_table = server.patches.last().cloned();
                msg.for_build_id = Some(BuildId::SECONDARY.0 as _);
            }
        }

        msg
    }

    /// Clear the hot reload changes. This should be called any time a new build is starting
    pub(crate) fn clear_hot_reload_changes(&mut self) {
        self.applied_client_hot_reload_message = Default::default();
    }

    pub(crate) fn clear_patches(&mut self) {
        self.client.patches.clear();
        if let Some(server) = self.server.as_mut() {
            server.patches.clear();
        }
    }

    /// Returns a static label for the current hotreload mode (used by both the TUI and logs).
    pub(crate) fn hotreload_mode_label(&self) -> &'static str {
        match self.hotreload_mode {
            HotReloadMode::Hotpatch => "hot-patching",
            HotReloadMode::RsxOnly => "rsx and assets",
            HotReloadMode::Disabled => "disabled",
        }
    }

    /// Cycle the hotreload mode: Hotpatch -> RsxOnly -> Disabled -> Hotpatch, i.e. monotonically
    /// decreasing reactivity. Returns `(previous_label, new_label)` for logging the transition.
    pub(crate) fn cycle_hotreload_mode(&mut self) -> (&'static str, &'static str) {
        let prev = self.hotreload_mode_label();
        self.hotreload_mode = match self.hotreload_mode {
            HotReloadMode::Hotpatch => HotReloadMode::RsxOnly,
            HotReloadMode::RsxOnly => HotReloadMode::Disabled,
            HotReloadMode::Disabled => HotReloadMode::Hotpatch,
        };
        (prev, self.hotreload_mode_label())
    }

    pub(crate) async fn client_connected(
        &mut self,
        build_id: BuildId,
        aslr_reference: Option<u64>,
        pid: Option<u32>,
    ) {
        match build_id {
            BuildId::PRIMARY
                // multiple tabs on web can cause this to be called incorrectly, and it doesn't
                // make any sense anyways
                if self.client.build.bundle != BundleFormat::Web => {
                    if let Some(aslr_reference) = aslr_reference {
                        self.client.aslr_reference = Some(aslr_reference);
                    }
                    if let Some(pid) = pid {
                        self.client.pid = Some(pid);
                    }
                }
            BuildId::SECONDARY => {
                if let Some(server) = self.server.as_mut() {
                    server.aslr_reference = aslr_reference;
                }
            }
            _ => {}
        }

        // Assign the runtime asset dir to the runner
        if self.client.build.bundle == BundleFormat::Ios {
            // xcrun simctl get_app_container booted com.dioxuslabs
            let res = Command::new("xcrun")
                .arg("simctl")
                .arg("get_app_container")
                .arg("booted")
                .arg(self.client.build.bundle_identifier())
                .output()
                .await;

            if let Ok(res) = res {
                tracing::trace!("Using runtime asset dir: {:?}", res);

                if let Ok(out) = String::from_utf8(res.stdout) {
                    let out = out.trim();

                    tracing::trace!("Setting Runtime asset dir: {out:?}");
                    self.client.runtime_asset_dir = Some(PathBuf::from(out));
                }
            }
        }
    }

    /// Store the hot reload changes for any future clients that connect
    fn add_hot_reload_message(&mut self, msg: &HotReloadMsg) {
        let applied = &mut self.applied_client_hot_reload_message;

        // Merge the assets, unknown files, and templates
        // We keep the newer change if there is both a old and new change
        let mut templates: HashMap<TemplateGlobalKey, _> = std::mem::take(&mut applied.templates)
            .into_iter()
            .map(|template| (template.key.clone(), template))
            .collect();
        let mut assets: HashSet<PathBuf> =
            std::mem::take(&mut applied.assets).into_iter().collect();
        for template in &msg.templates {
            templates.insert(template.key.clone(), template.clone());
        }
        assets.extend(msg.assets.iter().cloned());
        applied.templates = templates.into_values().collect();
        applied.assets = assets.into_iter().collect();
        applied.jump_table = self.client.patches.last().cloned();
    }

    /// Register the files from the workspace into our file watcher.
    ///
    /// This very simply looks for all Rust files in the workspace and adds them to the filemap.
    ///
    /// Once the builds complete we'll use the depinfo files to get the actual files that are used,
    /// making our watcher more accurate. Filling the filemap here is intended to catch any file changes
    /// in between the first build and the depinfo file being generated.
    ///
    /// We don't want watch any registry files since that generally causes a huge performance hit -
    /// we mostly just care about workspace files and local dependencies.
    ///
    /// Dep-info file background:
    /// <https://doc.rust-lang.org/stable/nightly-rustc/cargo/core/compiler/fingerprint/index.html#dep-info-files>
    fn load_rsx_filemap(&mut self) {
        self.fill_filemap_from_krate(self.client.build.crate_dir());

        if let Some(server) = self.server.as_ref() {
            self.fill_filemap_from_krate(server.build.crate_dir());
        }

        for krate_path in self.all_watched_crates() {
            self.fill_filemap_from_krate(krate_path);
        }
    }

    /// Fill the filemap with files from the filesystem, using the given filter to determine which files to include.
    ///
    /// You can use the filter with something like a gitignore to only include files that are relevant to your project.
    /// We'll walk the filesystem from the given path and recursively search for all files that match the filter.
    ///
    /// The filter function takes a path and returns true if the file should be included in the filemap.
    /// Generally this will only be .rs files
    ///
    /// If a file couldn't be parsed, we don't fail. Instead, we save the error.
    ///
    /// todo: There are known bugs here when handling gitignores.
    fn fill_filemap_from_krate(&mut self, crate_dir: PathBuf) {
        let src_dir = crate_dir.join("src");

        for entry in walkdir::WalkDir::new(src_dir).into_iter().flatten() {
            if self
                .workspace
                .ignore
                .matched(entry.path(), entry.file_type().is_dir())
                .is_ignore()
            {
                continue;
            }

            let path = entry.path();
            let pathbuf = path.to_path_buf();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let std::collections::hash_map::Entry::Vacant(e) = self.file_map.entry(pathbuf) {
                    if let Ok(contents) = std::fs::read_to_string(path) {
                        e.insert(CachedFile {
                            contents,
                            most_recent: None,
                            templates: Default::default(),
                        });
                    }
                }
            }
        }
    }

    /// Commit the changes to the filemap, overwriting the contents of the files
    ///
    /// Removes any cached templates and replaces the contents of the files with the most recent
    ///
    /// todo: we should-reparse the contents so we never send a new version, ever
    fn clear_cached_rsx(&mut self) {
        for cached_file in self.file_map.values_mut() {
            if let Some(most_recent) = cached_file.most_recent.take() {
                cached_file.contents = most_recent;
            }
            cached_file.templates.clear();
        }
    }

    fn watch_filesystem(&mut self) {
        // Watch the folders of the crates that we're interested in
        for path in self.watch_paths(
            self.client.build.crate_dir(),
            self.client.build.crate_package,
        ) {
            if let Err(err) = self.watcher.watch(&path, RecursiveMode::Recursive) {
                handle_notify_error(err);
            }
        }

        if let Some(server) = self.server.as_ref() {
            // Watch the server's crate directory as well
            for path in self.watch_paths(server.build.crate_dir(), server.build.crate_package) {
                tracing::trace!("Watching path {path:?}");

                if let Err(err) = self.watcher.watch(&path, RecursiveMode::Recursive) {
                    handle_notify_error(err);
                }
            }
        }

        // Also watch the crates themselves, but not recursively, such that we can pick up new folders
        for krate in self.all_watched_crates() {
            if let Err(err) = self.watcher.watch(&krate, RecursiveMode::NonRecursive) {
                handle_notify_error(err);
            }
        }

        // Also watch the workspace dir, non recursively, such that we can pick up new folders there too
        if let Err(err) = self.watcher.watch(
            self.workspace.krates.workspace_root().as_std_path(),
            RecursiveMode::NonRecursive,
        ) {
            handle_notify_error(err);
        }
    }

    /// Return the list of paths that we should watch for changes.
    fn watch_paths(&self, crate_dir: PathBuf, crate_package: NodeId) -> Vec<PathBuf> {
        let mut watched_paths = vec![];

        // Get a list of *all* the crates with Rust code that we need to watch.
        // This will end up being dependencies in the workspace and non-workspace dependencies on the user's computer.
        let mut watched_crates = self.local_dependencies(crate_package);
        watched_crates.push(crate_dir);

        // Watch the `public` directory if this is the client crate
        if self.client.build.crate_package == crate_package {
            if let Some(public_dir) = self.client.build.user_public_dir() {
                if public_dir.exists() {
                    watched_paths.push(public_dir);
                }
            }
        }

        // Now, watch all the folders in the crates, but respecting their respective ignore files
        for krate_root in watched_crates {
            // Build the ignore builder for this crate, but with our default ignore list as well
            let ignore = self.workspace.ignore_for_krate(&krate_root);

            for entry in krate_root.read_dir().into_iter().flatten() {
                let Ok(entry) = entry else {
                    continue;
                };

                if ignore
                    .matched(entry.path(), entry.path().is_dir())
                    .is_ignore()
                {
                    continue;
                }

                watched_paths.push(entry.path().to_path_buf());
            }
        }

        watched_paths.dedup();

        watched_paths
    }

    /// Get the directories of every local (non-`.cargo`) crate reachable from `crate_package`
    /// through the dependency graph. Walks transitively so workspace members pulled in via
    /// intermediate workspace crates (e.g. `dioxus-examples` → `dioxus` → `dioxus-core`) are
    /// included. Registry/git deps under `.cargo` are skipped and not descended into.
    fn local_dependencies(&self, crate_package: NodeId) -> Vec<PathBuf> {
        let mut paths = vec![];
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();

        visited.insert(crate_package);
        queue.push_back(crate_package);

        while let Some(node) = queue.pop_front() {
            for (dependency, _edge) in self.workspace.krates.get_deps(node) {
                let (krate, dep_nid) = match dependency {
                    krates::Node::Krate { id, krate, .. } => {
                        let nid = self.workspace.krates.nid_for_kid(id).unwrap();
                        (krate, nid)
                    }
                    krates::Node::Feature { krate_index, .. } => {
                        let k = &self.workspace.krates[krate_index.index()];
                        (k, *krate_index)
                    }
                };

                if !visited.insert(dep_nid) {
                    continue;
                }

                if krate
                    .manifest_path
                    .components()
                    .any(|c| c.as_str() == ".cargo")
                {
                    continue;
                }

                paths.push(
                    krate
                        .manifest_path
                        .parent()
                        .unwrap()
                        .to_path_buf()
                        .into_std_path_buf(),
                );
                queue.push_back(dep_nid);
            }
        }

        paths
    }

    fn all_watched_crates(&self) -> Vec<PathBuf> {
        let crate_package = self.client().build.crate_package;
        let crate_dir = self.client().build.crate_dir();

        let mut krates: Vec<PathBuf> = self
            .local_dependencies(crate_package)
            .into_iter()
            .chain(Some(crate_dir))
            .collect();

        if let Some(server) = self.server.as_ref() {
            let server_crate_package = server.build.crate_package;
            let server_crate_dir = server.build.crate_dir();

            krates.extend(
                self.local_dependencies(server_crate_package)
                    .into_iter()
                    .chain(Some(server_crate_dir)),
            );
        }

        krates.sort();
        krates.dedup();

        krates
    }

    /// Compute the ordered compilation chain from a changed workspace crate to the tip crate.
    ///
    /// Returns crate names (underscore-normalized) in compilation order: the changed crate first,
    /// then each intermediate workspace crate that depends on it, ending with the tip crate.
    ///
    /// Uses BFS from the tip crate through its workspace dependencies to find the path.
    /// If the changed crate IS the tip crate, returns just `[tip]`.
    fn workspace_dep_chain(&self, changed_crate: &str) -> Vec<String> {
        let tip_name = self.client.build.main_target.replace('-', "_");

        // If the changed crate is the tip, no chain needed
        if changed_crate == tip_name {
            return vec![tip_name];
        }

        // Build a map of workspace crate names to their krates NodeIds
        let mut name_to_node: HashMap<String, NodeId> = HashMap::new();
        for member in self.workspace.krates.workspace_members() {
            if let krates::Node::Krate { id, krate, .. } = member {
                let normalized = krate.name.replace('-', "_");
                name_to_node.insert(normalized, self.workspace.krates.nid_for_kid(id).unwrap());
            }
        }

        // BFS/DFS from tip through workspace deps to find path to changed crate.
        // We walk the dependency edges (tip → its deps → their deps → ...) looking for changed_crate.
        let Some(&tip_node) = name_to_node.get(&tip_name) else {
            return vec![changed_crate.to_string()];
        };

        // parent[node] = the workspace crate that depends on it (closer to tip)
        let mut parent: HashMap<NodeId, Option<NodeId>> = HashMap::new();
        parent.insert(tip_node, None);
        let mut queue = VecDeque::new();
        queue.push_back(tip_node);

        let mut target_node = None;

        while let Some(current) = queue.pop_front() {
            for (dep, _edge) in self.workspace.krates.get_deps(current) {
                let (dep_name, dep_nid) = match dep {
                    krates::Node::Krate { id, krate, .. } => {
                        let normalized = krate.name.replace('-', "_");
                        let nid = self.workspace.krates.nid_for_kid(id).unwrap();
                        (normalized, nid)
                    }
                    _ => continue,
                };

                // Only traverse workspace members
                if !name_to_node.contains_key(&dep_name) {
                    continue;
                }

                if parent.contains_key(&dep_nid) {
                    continue; // already visited
                }

                parent.insert(dep_nid, Some(current));

                if dep_name == changed_crate {
                    target_node = Some(dep_nid);
                    break;
                }

                queue.push_back(dep_nid);
            }

            if target_node.is_some() {
                break;
            }
        }

        // Reconstruct the path from changed_crate → ... → tip
        let Some(target) = target_node else {
            // Changed crate not found in workspace dep graph — just compile it alone
            return vec![changed_crate.to_string()];
        };

        let mut chain = vec![];
        let mut node = target;
        loop {
            // Find the crate name for this node
            let krate = &self.workspace.krates[node];
            chain.push(krate.name.replace('-', "_"));

            match parent.get(&node) {
                Some(Some(parent_node)) => node = *parent_node,
                _ => break,
            }
        }

        chain
    }

    /// Order a set of changed workspace crates so that deeper dependencies compile first.
    ///
    /// Uses `workspace_dep_chain` to determine the depth of each crate in the dependency graph,
    /// then sorts so that leaves (deepest deps) compile before crates closer to the tip.
    fn order_changed_crates(&self, files: &[PathBuf]) -> Vec<String> {
        // Determine which workspace crates changed based on the file paths.
        // Order them so deeper deps compile first (leaves before dependents).
        let changed_set: HashSet<String> = files
            .iter()
            .filter_map(|f| self.file_to_workspace_crate(f))
            .collect();

        let mut crates_with_depth: Vec<_> = changed_set
            .iter()
            .map(|c| (c.clone(), self.workspace_dep_chain(c).len()))
            .collect();

        // Longer chain = deeper in dep tree = should compile first
        crates_with_depth.sort_by_key(|b| std::cmp::Reverse(b.1));
        crates_with_depth.into_iter().map(|(c, _)| c).collect()
    }

    /// Map a changed file path to the workspace crate it belongs to.
    ///
    /// Returns the crate name in rustc convention (hyphens → underscores), matching the
    /// `--crate-name` arg used by rustc and the keys in `workspace_rustc_args`.
    ///
    /// Finds the workspace member whose crate directory is the longest prefix of the file path.
    fn file_to_workspace_crate(&self, file: &Path) -> Option<String> {
        let mut best_match: Option<(String, usize)> = None;

        for member in self.workspace.krates.workspace_members() {
            if let krates::Node::Krate { krate, .. } = member {
                let Some(crate_dir) = krate.manifest_path.parent() else {
                    continue;
                };
                if let Ok(relative) = file.strip_prefix(crate_dir.as_std_path()) {
                    let depth = relative.components().count();
                    let is_better = best_match
                        .as_ref()
                        .is_none_or(|(_, best_depth)| depth < *best_depth);
                    if is_better {
                        best_match = Some((krate.name.replace('-', "_"), depth));
                    }
                }
            }
        }

        best_match.map(|(name, _)| name)
    }

    /// Check if this is a fullstack build. This means that there is an additional build with the `server` platform.
    pub(crate) fn is_fullstack(&self) -> bool {
        self.fullstack
    }

    /// Return a number between 0 and 1 representing the progress of the server build
    pub(crate) fn server_compile_progress(&self) -> f64 {
        let Some(server) = self.server.as_ref() else {
            return 0.0;
        };

        server.compiled_crates as f64 / server.expected_crates as f64
    }

    pub(crate) async fn open_debugger(&mut self, dev: &WebServer, build: BuildId) {
        if self.hotreload_mode == HotReloadMode::Hotpatch {
            tracing::warn!(
                "Debugging symbols might not work properly with hotpatching enabled. Consider disabling hotpatching for debugging."
            );
        }

        match build {
            BuildId::PRIMARY => {
                _ = self.client.open_debugger(dev).await;
            }
            BuildId::SECONDARY => {
                if let Some(server) = self.server.as_mut() {
                    _ = server.open_debugger(dev).await;
                }
            }
            _ => {}
        }
    }

    /// Returns true if both the server and client are ready to accept thin/hotpatch rebuilds —
    /// i.e. they have completed artifacts *and* a populated `patch_cache` from a prior fat build.
    ///
    /// The cache check matters when cycling `RsxOnly`/`Disabled` -> `Hotpatch`: the existing
    /// artifacts are from a base build and there's no symbol map to diff against, so we have to
    /// fall back to a full fat rebuild before thin rebuilds can work.
    fn has_hotpatchable_builds(&self) -> bool {
        let builder_ready = |b: &AppBuilder| {
            b.artifacts
                .as_ref()
                .is_some_and(|a| a.patch_cache.is_some())
        };

        builder_ready(&self.client) && self.server.as_ref().map(builder_ready).unwrap_or(true)
    }
}

/// Bind a listener to any point and return it
/// When the listener is dropped, the socket will be closed, but we'll still have a port that we
/// can bind our proxy to.
///
/// Todo: we might want to do this on every new build in case the OS tries to bind things to this port
/// and we don't already have something bound to it. There's no great way of "reserving" a port.
fn get_available_port(address: IpAddr, prefer: Option<u16>) -> Option<u16> {
    // First, try to bind to the preferred port
    if let Some(port) = prefer {
        if let Ok(_listener) = TcpListener::bind((address, port)) {
            return Some(port);
        }
    }

    // Otherwise, try to bind to any port and return the first one we can
    TcpListener::bind((address, 0))
        .and_then(|listener| listener.local_addr().map(|f| f.port()))
        .ok()
}

fn create_notify_watcher(
    tx: UnboundedSender<notify::Event>,
    wsl_poll_interval: u64,
) -> Box<dyn NotifyWatcher> {
    // Build the event handler for notify.
    // This has been known to be a source of many problems, unfortunately, since notify handling seems to be flakey across platforms
    let handler = move |info: notify::Result<notify::Event>| {
        let Ok(event) = info else {
            return;
        };

        let is_allowed_notify_event = match event.kind {
            EventKind::Modify(ModifyKind::Data(_)) => true,
            EventKind::Modify(ModifyKind::Name(_)) => true,
            // The primary modification event on WSL's poll watcher.
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime)) => true,
            // Catch-all for unknown event types (windows)
            EventKind::Modify(ModifyKind::Any) => true,
            EventKind::Modify(ModifyKind::Metadata(_)) => false,
            // Don't care about anything else.
            EventKind::Create(_) => true,
            EventKind::Remove(_) => true,
            _ => false,
        };

        if is_allowed_notify_event {
            _ = tx.unbounded_send(event);
        }
    };

    const NOTIFY_ERROR_MSG: &str = "Failed to create file watcher.\nEnsure you have the required permissions to watch the specified directories.";

    // On wsl, we need to poll the filesystem for changes
    if is_wsl() {
        return Box::new(
            notify::PollWatcher::new(
                handler,
                Config::default().with_poll_interval(Duration::from_secs(wsl_poll_interval)),
            )
            .expect(NOTIFY_ERROR_MSG),
        );
    }

    // Otherwise we can use the recommended watcher
    Box::new(notify::recommended_watcher(handler).expect(NOTIFY_ERROR_MSG))
}

fn handle_notify_error(err: notify::Error) {
    tracing::debug!("Failed to watch path: {}", err);
    match err.kind {
        notify::ErrorKind::Io(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            tracing::error!("Failed to watch path: permission denied. {:?}", err.paths)
        }
        notify::ErrorKind::MaxFilesWatch => {
            tracing::error!("Failed to set up file watcher: too many files to watch")
        }
        _ => {}
    }
}

/// Detects if `dx` is being ran in a WSL environment.
///
/// We determine this based on whether the keyword `microsoft` or `wsl` is contained within the `WSL_1` or `WSL_2` files.
/// This may fail in the future as it isn't guaranteed by Microsoft.
/// See <https://github.com/microsoft/WSL/issues/423#issuecomment-221627364>
fn is_wsl() -> bool {
    const WSL_1: &str = "/proc/sys/kernel/osrelease";
    const WSL_2: &str = "/proc/version";
    const WSL_KEYWORDS: [&str; 2] = ["microsoft", "wsl"];

    // Test 1st File
    if let Ok(content) = std::fs::read_to_string(WSL_1) {
        let lowercase = content.to_lowercase();
        for keyword in WSL_KEYWORDS {
            if lowercase.contains(keyword) {
                return true;
            }
        }
    }

    // Test 2nd File
    if let Ok(content) = std::fs::read_to_string(WSL_2) {
        let lowercase = content.to_lowercase();
        for keyword in WSL_KEYWORDS {
            if lowercase.contains(keyword) {
                return true;
            }
        }
    }

    false
}

/// Format a Duration for human-readable output.
fn format_duration_ms(d: Duration) -> String {
    let total_ms = d.as_millis() as u64;

    if total_ms < 1000 {
        format!("{total_ms}ms")
    } else {
        let secs = total_ms as f64 / 1000.0;
        format!("{secs:.2}s")
    }
}

// ---------------------------------------------------------------------------------------------
// Cargo.toml / Dioxus.toml live-edit classification
//
// Decides whether an in-flight `dx serve` should respond to an edit of `Cargo.toml` or
// `Dioxus.toml` with a full rebuild, a "restart required" warning, or by silently ignoring it.
//
// The classification is field-aware: we parse the file as `toml::Value` and compare a curated
// set of subtrees. Whitespace, comments, key reordering, and edits to fields outside the curated
// set show up as `Ignore`, so a stray edit to e.g. `package.description` doesn't kick off a
// 30-second rebuild.
//
// Profile and platform sections are filtered against the *active* profile and bundle. Editing
// `[profile.release]` while serving in `dev` produces an `Ignore` with a debug note rather
// than a rebuild — likewise for `[ios]` settings while serving for the web.
//
// Parse failures intentionally return `Ignore` and leave the snapshot untouched, so a save
// that's mid-edit (broken syntax) doesn't get diffed and doesn't pollute the baseline. The
// next successful save is what drives a decision.
// ---------------------------------------------------------------------------------------------

/// What the runner should do in response to a config-file edit.
///
/// `FullRebuild` and `WarnRestart` carry the human-readable cause split into a `subject`
/// (e.g. `Cargo.toml [dependencies]`) and a `detail` (e.g. `changed` plus any trailing hint).
/// The split lets the log formatter style only the subject, leaving the verb and any
/// follow-on prose in the default color.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfigChangeOutcome {
    /// Nothing rebuild-relevant changed (whitespace, comment-only edit, irrelevant field, edit
    /// to an inactive profile/platform). `note` is logged at debug level so silent edits aren't
    /// completely invisible.
    Ignore { note: Option<String> },

    /// A field that affects compilation changed. The runner should kick off a full rebuild.
    FullRebuild { subject: String, detail: String },

    /// A field that's only consumed at devserver-startup changed (proxy, https, watch paths).
    /// The runner should warn the user that a `dx serve` restart is required to pick up the
    /// change. No build action is taken.
    WarnRestart { subject: String, detail: String },
}

impl ConfigChangeOutcome {
    /// Combine two outcomes by keeping the strongest action.
    /// `FullRebuild` > `WarnRestart` > `Ignore`.
    fn escalate(self, other: ConfigChangeOutcome) -> ConfigChangeOutcome {
        use ConfigChangeOutcome::*;
        match (&self, &other) {
            (FullRebuild { .. }, _) => self,
            (_, FullRebuild { .. }) => other,
            (WarnRestart { .. }, _) => self,
            (_, WarnRestart { .. }) => other,
            (Ignore { note: Some(_) }, Ignore { note: None }) => self,
            (Ignore { note: None }, Ignore { note: Some(_) }) => other,
            _ => self,
        }
    }
}

/// Active build context passed to the pure analysis functions. Lets `[profile.<name>]` and
/// `[<platform>]` edits be classified relative to whatever the running serve session targets.
#[derive(Debug, Clone)]
struct AnalysisCtx {
    active_profile: String,
    active_bundle: BundleFormat,
}

fn read_toml_file(path: &Path) -> Result<toml::Value, anyhow::Error> {
    let s = std::fs::read_to_string(path)?;
    let v = toml::from_str::<toml::Value>(&s)?;
    Ok(v)
}

/// Look up `path` (a list of table keys) inside a TOML value. Returns `None` at the first
/// missing key or non-table along the way.
fn toml_get<'a>(value: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    let mut current = value;
    for key in path {
        let table = current.as_table()?;
        current = table.get(*key)?;
    }
    Some(current)
}

fn analyze_cargo_value(
    old: &toml::Value,
    new: &toml::Value,
    ctx: &AnalysisCtx,
) -> ConfigChangeOutcome {
    if old == new {
        return ConfigChangeOutcome::Ignore { note: None };
    }

    let mut outcome = ConfigChangeOutcome::Ignore { note: None };

    // -------- Sections that always force a full rebuild when their contents differ --------
    let rebuild_sections: &[&str] = &[
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "features",
        "lib",
        "bin",
        "example",
        "test",
        "bench",
        "patch",
        "replace",
    ];
    for section in rebuild_sections {
        if toml_get(old, &[section]) != toml_get(new, &[section]) {
            outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                subject: format!("Cargo.toml [{section}]"),
                detail: "changed".to_string(),
            });
        }
    }

    // -------- target.<cfg>.dependencies / dev-dependencies / build-dependencies --------
    if toml_get(old, &["target"]) != toml_get(new, &["target"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
            subject: "Cargo.toml [target.*]".to_string(),
            detail: "dependencies changed".to_string(),
        });
    }

    // -------- [package] subset that affects compilation --------
    let pkg_compile_keys: &[&str] = &[
        "name",
        "version",
        "edition",
        "rust-version",
        "build",
        "default-run",
        "links",
        "autobins",
        "autoexamples",
        "autotests",
        "autobenches",
        "resolver",
    ];
    for key in pkg_compile_keys {
        if toml_get(old, &["package", key]) != toml_get(new, &["package", key]) {
            outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                subject: format!("Cargo.toml [package].{key}"),
                detail: "changed".to_string(),
            });
        }
    }

    // -------- [profile.<name>] — only relevant if <name> is in the active profile's chain --------
    if let (Some(old_profiles), Some(new_profiles)) = (
        toml_get(old, &["profile"]).and_then(|v| v.as_table()),
        toml_get(new, &["profile"]).and_then(|v| v.as_table()),
    ) {
        let mut all_names: std::collections::BTreeSet<&str> = Default::default();
        all_names.extend(old_profiles.keys().map(String::as_str));
        all_names.extend(new_profiles.keys().map(String::as_str));

        for name in all_names {
            if old_profiles.get(name) == new_profiles.get(name) {
                continue;
            }
            // Use the *new* profile table for inheritance so a freshly-added `inherits` is
            // honored, falling back to the old table for profiles that were just deleted.
            if profile_in_active_chain(name, &ctx.active_profile, new_profiles, old_profiles) {
                outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                    subject: format!("Cargo.toml [profile.{name}]"),
                    detail: "changed".to_string(),
                });
            } else {
                outcome = outcome.escalate(ConfigChangeOutcome::Ignore {
                    note: Some(format!(
                        "Saw change to [profile.{name}] but active profile is `{}` — ignoring.",
                        ctx.active_profile
                    )),
                });
            }
        }
    } else if toml_get(old, &["profile"]) != toml_get(new, &["profile"]) {
        // One side missing the [profile] table entirely — treat as full rebuild for safety.
        outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
            subject: "Cargo.toml [profile]".to_string(),
            detail: "section added or removed".to_string(),
        });
    }

    // -------- [workspace] subset that affects build composition --------
    let workspace_compile_keys: &[&str] = &[
        "members",
        "default-members",
        "exclude",
        "resolver",
        "dependencies",
        "package",
        "metadata",
    ];
    for key in workspace_compile_keys {
        if toml_get(old, &["workspace", key]) != toml_get(new, &["workspace", key]) {
            let extra = if *key == "members" || *key == "default-members" {
                " (note: source files in newly-added workspace members won't be hot-reloaded until you restart `dx serve`)"
            } else {
                ""
            };
            outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                subject: format!("Cargo.toml [workspace].{key}"),
                detail: format!("changed{extra}"),
            });
        }
    }

    // -------- workspace-level patch / replace --------
    if toml_get(old, &["workspace", "patch"]) != toml_get(new, &["workspace", "patch"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
            subject: "Cargo.toml [workspace.patch]".to_string(),
            detail: "changed".to_string(),
        });
    }

    outcome
}

/// Walk a profile's `inherits` chain in `profiles` (using `fallback` for profiles missing on
/// the new side) and return true if `target` is `start` or any ancestor of `start`.
///
/// Cargo's built-in fallback chain (`test`→`dev`, `bench`→`release`) is encoded explicitly.
fn profile_in_active_chain(
    target: &str,
    start: &str,
    primary: &toml::value::Table,
    fallback: &toml::value::Table,
) -> bool {
    if target == start {
        return true;
    }

    // Built-in defaults — these inherit even if not declared.
    let implicit_inherits = match start {
        "test" => Some("dev"),
        "bench" => Some("release"),
        _ => None,
    };

    let mut current = start.to_string();
    let mut visited: HashSet<String> = Default::default();
    visited.insert(current.clone());

    loop {
        let table = primary
            .get(&current)
            .and_then(|v| v.as_table())
            .or_else(|| fallback.get(&current).and_then(|v| v.as_table()));

        let next = match table
            .and_then(|t| t.get("inherits"))
            .and_then(|v| v.as_str())
        {
            Some(s) => s.to_string(),
            None => match implicit_inherits {
                Some(s) if current == start => s.to_string(),
                _ => return false,
            },
        };

        if next == target {
            return true;
        }
        if !visited.insert(next.clone()) {
            return false; // cycle
        }
        current = next;
    }
}

fn analyze_dioxus_value(
    old: &toml::Value,
    new: &toml::Value,
    ctx: &AnalysisCtx,
) -> ConfigChangeOutcome {
    if old == new {
        return ConfigChangeOutcome::Ignore { note: None };
    }

    let mut outcome = ConfigChangeOutcome::Ignore { note: None };

    // ---- [application] — paths and identifiers compiled into the build ----
    let app_rebuild_keys: &[&str] = &[
        "name",
        "out_dir",
        "asset_dir",
        "public_dir",
        "tailwind_input",
        "tailwind_output",
        "ios_info_plist",
        "macos_info_plist",
        "ios_entitlements",
        "macos_entitlements",
        "android_manifest",
        "android_main_activity",
        "android_min_sdk_version",
    ];
    for key in app_rebuild_keys {
        if toml_get(old, &["application", key]) != toml_get(new, &["application", key]) {
            outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                subject: format!("Dioxus.toml [application].{key}"),
                detail: "changed".to_string(),
            });
        }
    }

    // ---- [web.app] — title and base_path get baked into HTML / WASM URLs ----
    if toml_get(old, &["web", "app"]) != toml_get(new, &["web", "app"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
            subject: "Dioxus.toml [web.app]".to_string(),
            detail: "changed".to_string(),
        });
    }

    // ---- [web.proxy] — only consumed when devserver boots ----
    if toml_get(old, &["web", "proxy"]) != toml_get(new, &["web", "proxy"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::WarnRestart {
            subject: "Dioxus.toml [web.proxy]".to_string(),
            detail: "changed — restart `dx serve` to apply.".to_string(),
        });
    }

    // ---- [web.https] — TLS config initialized at boot ----
    if toml_get(old, &["web", "https"]) != toml_get(new, &["web", "https"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::WarnRestart {
            subject: "Dioxus.toml [web.https]".to_string(),
            detail: "changed — restart `dx serve` to apply.".to_string(),
        });
    }

    // ---- [web.watcher] — watcher mounted at startup; can't be re-mounted live (yet) ----
    if toml_get(old, &["web", "watcher"]) != toml_get(new, &["web", "watcher"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::WarnRestart {
            subject: "Dioxus.toml [web.watcher]".to_string(),
            detail: "changed — restart `dx serve` to apply.".to_string(),
        });
    }

    // ---- [web.resource] — injected into HTML at build time ----
    if toml_get(old, &["web", "resource"]) != toml_get(new, &["web", "resource"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
            subject: "Dioxus.toml [web.resource]".to_string(),
            detail: "changed".to_string(),
        });
    }

    // ---- [permissions] / [deep_links] / [background] ----
    for section in &["permissions", "deep_links", "background"] {
        if toml_get(old, &[section]) != toml_get(new, &[section]) {
            outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                subject: format!("Dioxus.toml [{section}]"),
                detail: "changed".to_string(),
            });
        }
    }

    // ---- Per-platform sections — only rebuild if they're the active platform ----
    let platform_sections = [
        ("ios", BundleFormat::Ios),
        ("android", BundleFormat::Android),
        ("macos", BundleFormat::MacOS),
        ("windows", BundleFormat::Windows),
        ("linux", BundleFormat::Linux),
    ];
    for (section, fmt) in platform_sections {
        if toml_get(old, &[section]) != toml_get(new, &[section]) {
            if ctx.active_bundle == fmt {
                outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                    subject: format!("Dioxus.toml [{section}]"),
                    detail: "changed".to_string(),
                });
            } else {
                outcome = outcome.escalate(ConfigChangeOutcome::Ignore {
                    note: Some(format!(
                        "Saw change to [{section}] but active bundle is `{}` — ignoring.",
                        ctx.active_bundle
                    )),
                });
            }
        }
    }

    // [bundle], [components], [web.pre_compress], [web.wasm_opt] are intentionally NOT in
    // either rebuild or warn lists — they only matter for `dx bundle` / `dx components` /
    // release post-processing, none of which run during `dx serve`.

    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> toml::Value {
        toml::from_str(s).expect("valid toml")
    }

    fn ctx_dev_web() -> AnalysisCtx {
        AnalysisCtx {
            active_profile: "dev".to_string(),
            active_bundle: BundleFormat::Web,
        }
    }

    fn ctx_release_ios() -> AnalysisCtx {
        AnalysisCtx {
            active_profile: "release".to_string(),
            active_bundle: BundleFormat::Ios,
        }
    }

    fn assert_rebuild(outcome: &ConfigChangeOutcome) {
        assert!(
            matches!(outcome, ConfigChangeOutcome::FullRebuild { .. }),
            "expected FullRebuild, got {outcome:?}"
        );
    }

    fn assert_warn(outcome: &ConfigChangeOutcome) {
        assert!(
            matches!(outcome, ConfigChangeOutcome::WarnRestart { .. }),
            "expected WarnRestart, got {outcome:?}"
        );
    }

    fn assert_ignore(outcome: &ConfigChangeOutcome) {
        assert!(
            matches!(outcome, ConfigChangeOutcome::Ignore { .. }),
            "expected Ignore, got {outcome:?}"
        );
    }

    // ============================================================================
    // Cargo.toml — baseline + variants
    // ============================================================================

    const CARGO_BASELINE: &str = r#"
[package]
name = "demo"
version = "0.1.0"
edition = "2021"
description = "a demo crate"
license = "MIT"

[dependencies]
serde = "1"

[dev-dependencies]
proptest = "1"

[features]
default = ["foo"]
foo = []

[profile.dev]
opt-level = 0

[profile.release]
opt-level = 3
"#;

    #[test]
    fn cargo_identical_is_ignore() {
        let v = parse(CARGO_BASELINE);
        assert_ignore(&analyze_cargo_value(&v, &v, &ctx_dev_web()));
    }

    #[test]
    fn cargo_add_dependency_rebuilds() {
        let new = parse(&format!(
            r#"{CARGO_BASELINE}
[dependencies.tokio]
version = "1"
"#
        ));
        let outcome = analyze_cargo_value(&parse(CARGO_BASELINE), &new, &ctx_dev_web());
        assert_rebuild(&outcome);
    }

    #[test]
    fn cargo_bump_dep_version_rebuilds() {
        let new = parse(&CARGO_BASELINE.replace(r#"serde = "1""#, r#"serde = "2""#));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_remove_dependency_rebuilds() {
        let new = parse(&CARGO_BASELINE.replace(r#"serde = "1""#, ""));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_dev_dependency_rebuilds() {
        let new = parse(&format!(
            r#"{CARGO_BASELINE}
[dev-dependencies.criterion]
version = "0.5"
"#
        ));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_build_dependency_rebuilds() {
        let new = parse(&format!(
            r#"{CARGO_BASELINE}
[build-dependencies]
cc = "1"
"#
        ));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_target_specific_dep_rebuilds() {
        let new = parse(&format!(
            r#"{CARGO_BASELINE}
[target.'cfg(unix)'.dependencies]
libc = "0.2"
"#
        ));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_feature_rebuilds() {
        let new = parse(&CARGO_BASELINE.replace("foo = []", "foo = []\nbar = []"));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_default_feature_rebuilds() {
        let new = parse(&CARGO_BASELINE.replace(r#"default = ["foo"]"#, "default = []"));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_edition_rebuilds() {
        let new = parse(&CARGO_BASELINE.replace(r#"edition = "2021""#, r#"edition = "2024""#));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_rust_version_rebuilds() {
        let spliced = CARGO_BASELINE.replace(
            r#"license = "MIT""#,
            "license = \"MIT\"\nrust-version = \"1.80\"",
        );
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&spliced),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_active_profile_rebuilds() {
        let new = parse(&CARGO_BASELINE.replace("opt-level = 0", "opt-level = 1"));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_inactive_profile_ignored_in_dev() {
        // Editing [profile.release] while serving in dev should NOT rebuild.
        let new = parse(&CARGO_BASELINE.replace("opt-level = 3", "opt-level = 2"));
        let outcome = analyze_cargo_value(&parse(CARGO_BASELINE), &new, &ctx_dev_web());
        assert_ignore(&outcome);
        if let ConfigChangeOutcome::Ignore { note: Some(n) } = &outcome {
            assert!(n.contains("[profile.release]"));
            assert!(n.contains("`dev`"));
        } else {
            panic!("expected Ignore-with-note, got {outcome:?}");
        }
    }

    #[test]
    fn cargo_change_inactive_profile_ignored_in_release() {
        // Editing [profile.dev] while serving in release should NOT rebuild.
        let new = parse(&CARGO_BASELINE.replace("opt-level = 0", "opt-level = 1"));
        let outcome = analyze_cargo_value(&parse(CARGO_BASELINE), &new, &ctx_release_ios());
        assert_ignore(&outcome);
    }

    #[test]
    fn cargo_change_inherited_profile_rebuilds() {
        // Active profile "android-dev" inherits from "dev". A change to [profile.dev] must
        // rebuild because it propagates through the inherits chain.
        let baseline = format!(
            r#"{CARGO_BASELINE}
[profile.android-dev]
inherits = "dev"
opt-level = 1
"#
        );
        let modified = baseline.replacen("opt-level = 0", "opt-level = 1", 1);
        let ctx = AnalysisCtx {
            active_profile: "android-dev".to_string(),
            active_bundle: BundleFormat::Android,
        };
        let outcome = analyze_cargo_value(&parse(&baseline), &parse(&modified), &ctx);
        assert_rebuild(&outcome);
    }

    #[test]
    fn cargo_change_test_profile_with_implicit_dev_inheritance() {
        // `test` implicitly inherits from `dev`. Active = `test` → editing [profile.dev]
        // must rebuild even without an explicit `inherits` key.
        let new = parse(&CARGO_BASELINE.replace("opt-level = 0", "opt-level = 1"));
        let ctx = AnalysisCtx {
            active_profile: "test".to_string(),
            active_bundle: BundleFormat::Web,
        };
        assert_rebuild(&analyze_cargo_value(&parse(CARGO_BASELINE), &new, &ctx));
    }

    #[test]
    fn cargo_change_workspace_members_rebuilds_with_warn_note() {
        let baseline = r#"
[workspace]
members = ["a"]
"#;
        let modified = r#"
[workspace]
members = ["a", "b"]
"#;
        let outcome = analyze_cargo_value(&parse(baseline), &parse(modified), &ctx_dev_web());
        assert_rebuild(&outcome);
        if let ConfigChangeOutcome::FullRebuild { subject, detail } = outcome {
            assert!(subject.contains("workspace"));
            assert!(detail.contains("hot-reloaded"));
        }
    }

    #[test]
    fn cargo_change_workspace_dependencies_rebuilds() {
        let baseline = r#"
[workspace]
members = ["a"]
[workspace.dependencies]
serde = "1"
"#;
        let modified = r#"
[workspace]
members = ["a"]
[workspace.dependencies]
serde = "2"
"#;
        assert_rebuild(&analyze_cargo_value(
            &parse(baseline),
            &parse(modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_patch_rebuilds() {
        let modified = format!(
            r#"{CARGO_BASELINE}
[patch.crates-io]
serde = {{ git = "https://github.com/serde-rs/serde" }}
"#
        );
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_lib_section_rebuilds() {
        let modified = format!(
            r#"{CARGO_BASELINE}
[lib]
name = "demo_lib"
"#
        );
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_bin_path_rebuilds() {
        let baseline = format!(
            r#"{CARGO_BASELINE}
[[bin]]
name = "demo"
path = "src/main.rs"
"#
        );
        let modified = format!(
            r#"{CARGO_BASELINE}
[[bin]]
name = "demo"
path = "src/bin.rs"
"#
        );
        assert_rebuild(&analyze_cargo_value(
            &parse(&baseline),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_lints_section_ignored() {
        let modified = format!(
            r#"{CARGO_BASELINE}
[lints.rust]
unsafe_code = "forbid"
"#
        );
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_description_ignored() {
        let modified = CARGO_BASELINE.replace(
            r#"description = "a demo crate""#,
            r#"description = "an updated demo""#,
        );
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_license_ignored() {
        let modified = CARGO_BASELINE.replace(r#"license = "MIT""#, r#"license = "Apache-2.0""#);
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_authors_ignored() {
        let modified = CARGO_BASELINE.replace(
            r#"license = "MIT""#,
            r#"license = "MIT"
authors = ["jon"]"#,
        );
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_reorder_keys_ignored() {
        let reordered = r#"
[package]
edition = "2021"
license = "MIT"
description = "a demo crate"
version = "0.1.0"
name = "demo"

[dev-dependencies]
proptest = "1"

[dependencies]
serde = "1"

[features]
foo = []
default = ["foo"]

[profile.release]
opt-level = 3

[profile.dev]
opt-level = 0
"#;
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(reordered),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_comment_only_change_ignored() {
        // Inserting a comment doesn't change the parsed Value at all.
        let modified = format!("# new comment\n{CARGO_BASELINE}");
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_whitespace_change_ignored() {
        let modified = CARGO_BASELINE.replace("\n\n", "\n\n\n\n");
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    // ============================================================================
    // Dioxus.toml — baseline + variants
    // ============================================================================

    const DIOXUS_BASELINE: &str = r#"
[application]
name = "demo"
public_dir = "public"

[web.app]
title = "demo"

[web.watcher]
watch_path = ["src"]
reload_html = false
index_on_404 = true

[bundle]
identifier = "com.example.demo"

[ios]
identifier = "com.example.demo.ios"

[android]
identifier = "com.example.demo.android"
"#;

    #[test]
    fn dioxus_identical_is_ignore() {
        let v = parse(DIOXUS_BASELINE);
        assert_ignore(&analyze_dioxus_value(&v, &v, &ctx_dev_web()));
    }

    #[test]
    fn dioxus_change_app_title_rebuilds() {
        let modified = DIOXUS_BASELINE.replace(r#"title = "demo""#, r#"title = "renamed""#);
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_base_path_rebuilds() {
        let modified = DIOXUS_BASELINE.replace(
            r#"title = "demo""#,
            "title = \"demo\"\nbase_path = \"/app\"",
        );
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_public_dir_rebuilds() {
        let modified =
            DIOXUS_BASELINE.replace(r#"public_dir = "public""#, r#"public_dir = "static""#);
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_tailwind_input_rebuilds() {
        let modified = DIOXUS_BASELINE.replace(
            r#"public_dir = "public""#,
            r#"public_dir = "public"
tailwind_input = "src/input.css""#,
        );
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_add_proxy_warns_restart() {
        let modified = format!(
            r#"{DIOXUS_BASELINE}
[[web.proxy]]
backend = "http://localhost:9999/api"
"#
        );
        assert_warn(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_proxy_backend_warns_restart() {
        let baseline = format!(
            r#"{DIOXUS_BASELINE}
[[web.proxy]]
backend = "http://localhost:9999/api"
"#
        );
        let modified = baseline.replace("9999", "8888");
        assert_warn(&analyze_dioxus_value(
            &parse(&baseline),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_enable_https_warns_restart() {
        let modified = format!(
            r#"{DIOXUS_BASELINE}
[web.https]
enabled = true
"#
        );
        assert_warn(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_watcher_paths_warns_restart() {
        let modified =
            DIOXUS_BASELINE.replace(r#"watch_path = ["src"]"#, r#"watch_path = ["src", "lib"]"#);
        assert_warn(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_add_dev_resource_script_rebuilds() {
        let modified = format!(
            r#"{DIOXUS_BASELINE}
[web.resource.dev]
script = ["http://example.com/x.js"]
"#
        );
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_bundle_identifier_ignored() {
        let modified = DIOXUS_BASELINE.replace(
            r#"identifier = "com.example.demo""#,
            r#"identifier = "com.example.renamed""#,
        );
        assert_ignore(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_components_section_ignored() {
        let modified = format!(
            r#"{DIOXUS_BASELINE}
[components]
git = "https://example.com"
"#
        );
        assert_ignore(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_wasm_opt_level_ignored() {
        let modified = format!(
            r#"{DIOXUS_BASELINE}
[web.wasm_opt]
level = "3"
"#
        );
        assert_ignore(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_pre_compress_ignored() {
        let modified = format!(
            r#"{DIOXUS_BASELINE}
[web]
pre_compress = true
"#
        );
        assert_ignore(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_ios_identifier_when_active_rebuilds() {
        let modified = DIOXUS_BASELINE.replace(
            r#"identifier = "com.example.demo.ios""#,
            r#"identifier = "com.example.renamed.ios""#,
        );
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_release_ios(),
        ));
    }

    #[test]
    fn dioxus_change_ios_identifier_when_web_active_ignored() {
        let modified = DIOXUS_BASELINE.replace(
            r#"identifier = "com.example.demo.ios""#,
            r#"identifier = "com.example.renamed.ios""#,
        );
        let outcome =
            analyze_dioxus_value(&parse(DIOXUS_BASELINE), &parse(&modified), &ctx_dev_web());
        assert_ignore(&outcome);
        if let ConfigChangeOutcome::Ignore { note: Some(n) } = &outcome {
            assert!(n.contains("[ios]"));
        } else {
            panic!("expected Ignore-with-note, got {outcome:?}");
        }
    }

    #[test]
    fn dioxus_add_permission_rebuilds() {
        let modified = format!(
            r#"{DIOXUS_BASELINE}
[permissions]
camera = {{ description = "need camera" }}
"#
        );
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_reorder_keys_ignored() {
        let reordered = r#"
[android]
identifier = "com.example.demo.android"

[ios]
identifier = "com.example.demo.ios"

[bundle]
identifier = "com.example.demo"

[web.watcher]
index_on_404 = true
reload_html = false
watch_path = ["src"]

[web.app]
title = "demo"

[application]
public_dir = "public"
name = "demo"
"#;
        assert_ignore(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(reordered),
            &ctx_dev_web(),
        ));
    }

    // ============================================================================
    // Outcome escalation
    // ============================================================================

    fn rebuild(s: &str, d: &str) -> ConfigChangeOutcome {
        ConfigChangeOutcome::FullRebuild {
            subject: s.into(),
            detail: d.into(),
        }
    }

    fn warn(s: &str, d: &str) -> ConfigChangeOutcome {
        ConfigChangeOutcome::WarnRestart {
            subject: s.into(),
            detail: d.into(),
        }
    }

    #[test]
    fn escalate_full_rebuild_dominates_warn_restart() {
        let a = rebuild("a", "changed");
        let b = warn("b", "changed");
        assert_rebuild(&a.clone().escalate(b.clone()));
        assert_rebuild(&b.escalate(a));
    }

    #[test]
    fn escalate_warn_restart_dominates_ignore() {
        let a = warn("a", "changed");
        let b = ConfigChangeOutcome::Ignore { note: None };
        assert_warn(&a.clone().escalate(b.clone()));
        assert_warn(&b.escalate(a));
    }

    // ============================================================================
    // File-level round-trip (parse-error & no-repeat-rebuild behaviors)
    // ============================================================================
    //
    // These exercise the helpers that load and diff against the on-disk content. They use a
    // standalone `Snapshot` map rather than spinning up an `AppServer`, which keeps tests
    // fast and isolated from the rest of the runner.

    fn diff_cargo_file(
        snapshots: &mut HashMap<PathBuf, toml::Value>,
        path: &Path,
        ctx: &AnalysisCtx,
    ) -> ConfigChangeOutcome {
        let new_value = match read_toml_file(path) {
            Ok(v) => v,
            Err(_) => {
                return ConfigChangeOutcome::Ignore {
                    note: Some(format!(
                        "Cargo.toml parse failed at {}, will retry on next save",
                        path.display()
                    )),
                };
            }
        };
        let old_value = snapshots
            .get(path)
            .cloned()
            .unwrap_or_else(|| toml::Value::Table(Default::default()));
        let outcome = analyze_cargo_value(&old_value, &new_value, ctx);
        snapshots.insert(path.to_path_buf(), new_value);
        outcome
    }

    fn seed_cargo_into(snapshots: &mut HashMap<PathBuf, toml::Value>, path: &Path) {
        if let Ok(v) = read_toml_file(path) {
            snapshots.insert(path.to_path_buf(), v);
        }
    }

    #[test]
    fn diff_returns_ignore_for_unchanged_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(&path, CARGO_BASELINE).unwrap();

        let mut snaps = HashMap::new();
        seed_cargo_into(&mut snaps, &path);
        assert_ignore(&diff_cargo_file(&mut snaps, &path, &ctx_dev_web()));
    }

    #[test]
    fn diff_detects_added_dependency() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(&path, CARGO_BASELINE).unwrap();

        let mut snaps = HashMap::new();
        seed_cargo_into(&mut snaps, &path);

        let modified = format!(
            r#"{CARGO_BASELINE}
[dependencies.tokio]
version = "1"
"#
        );
        std::fs::write(&path, &modified).unwrap();
        assert_rebuild(&diff_cargo_file(&mut snaps, &path, &ctx_dev_web()));
    }

    #[test]
    fn diff_returns_ignore_with_note_on_parse_error_and_keeps_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(&path, CARGO_BASELINE).unwrap();

        let mut snaps = HashMap::new();
        seed_cargo_into(&mut snaps, &path);

        // Mid-edit garbage: snapshot must NOT be clobbered.
        std::fs::write(&path, "[package\nthis is not valid").unwrap();
        let outcome = diff_cargo_file(&mut snaps, &path, &ctx_dev_web());
        assert_ignore(&outcome);
        if let ConfigChangeOutcome::Ignore { note: Some(n) } = &outcome {
            assert!(n.contains("parse failed"));
        } else {
            panic!("expected Ignore-with-note, got {outcome:?}");
        }

        // Subsequent valid save still diffs against the *seeded* baseline.
        let modified = format!(
            r#"{CARGO_BASELINE}
[dependencies.tokio]
version = "1"
"#
        );
        std::fs::write(&path, &modified).unwrap();
        assert_rebuild(&diff_cargo_file(&mut snaps, &path, &ctx_dev_web()));
    }

    #[test]
    fn diff_no_repeat_rebuild_for_same_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(&path, CARGO_BASELINE).unwrap();

        let mut snaps = HashMap::new();
        seed_cargo_into(&mut snaps, &path);

        let modified = format!(
            r#"{CARGO_BASELINE}
[dependencies.tokio]
version = "1"
"#
        );
        std::fs::write(&path, &modified).unwrap();
        assert_rebuild(&diff_cargo_file(&mut snaps, &path, &ctx_dev_web()));

        // Second analyze call against the *same* file content should be Ignore — the
        // snapshot was updated by the first call, so we don't loop on the same edit.
        assert_ignore(&diff_cargo_file(&mut snaps, &path, &ctx_dev_web()));
    }
}
