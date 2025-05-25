use super::{AppBuilder, ServeUpdate, WebServer};
use crate::{
    BuildArtifacts, BuildId, BuildMode, BuildTargets, Error, HotpatchModuleCache, Platform, Result,
    ServeArgs, TailwindCli, TraceSrc, Workspace,
};
use anyhow::Context;
use dioxus_core::internal::{
    HotReloadTemplateWithLocation, HotReloadedTemplate, TemplateGlobalKey,
};
use dioxus_devtools_types::HotReloadMsg;
use dioxus_dx_wire_format::BuildStage;
use dioxus_html::HtmlCtx;
use dioxus_rsx::CallBody;
use dioxus_rsx_hotreload::{ChangedRsx, HotReloadResult};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::future::OptionFuture;
use futures_util::StreamExt;
use krates::NodeId;
use notify::{
    event::{MetadataKind, ModifyKind},
    Config, EventKind, RecursiveMode, Watcher as NotifyWatcher,
};
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, TcpListener},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use subsecond_types::JumpTable;
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

    // Related to to the filesystem watcher
    pub(crate) watcher: Box<dyn notify::Watcher>,
    pub(crate) _watcher_tx: UnboundedSender<notify::Event>,
    pub(crate) watcher_rx: UnboundedReceiver<notify::Event>,

    // Tracked state related to open builds and hot reloading
    pub(crate) applied_hot_reload_message: HotReloadMsg,
    pub(crate) file_map: HashMap<PathBuf, CachedFile>,

    // Resolved args related to how we go about processing the rebuilds and logging
    pub(crate) use_hotpatch_engine: bool,
    pub(crate) automatic_rebuilds: bool,
    pub(crate) interactive: bool,
    pub(crate) _force_sequential: bool,
    pub(crate) hot_reload: bool,
    pub(crate) open_browser: bool,
    pub(crate) _wsl_file_poll_interval: u16,
    pub(crate) always_on_top: bool,
    pub(crate) fullstack: bool,
    pub(crate) watch_fs: bool,

    // resolve args related to the webserver
    pub(crate) devserver_port: u16,
    pub(crate) devserver_bind_ip: IpAddr,
    pub(crate) proxied_port: Option<u16>,
    pub(crate) cross_origin_policy: bool,

    // Additional plugin-type tools
    pub(crate) tw_watcher: tokio::task::JoinHandle<Result<()>>,
}

pub(crate) struct CachedFile {
    contents: String,
    most_recent: Option<String>,
    templates: HashMap<TemplateGlobalKey, HotReloadedTemplate>,
}

impl AppServer {
    /// Create the AppRunner and then initialize the filemap with the crate directory.
    pub(crate) async fn start(args: ServeArgs) -> Result<Self> {
        let workspace = Workspace::current().await?;

        // Resolve the simpler args
        let interactive = args.is_interactive_tty();
        let force_sequential = args.force_sequential;
        let cross_origin_policy = args.cross_origin_policy;

        // These come from the args but also might come from the workspace settings
        // We opt to use the manually specified args over the workspace settings
        let hot_reload = args
            .hot_reload
            .unwrap_or_else(|| workspace.settings.always_hot_reload.unwrap_or(true));

        let open_browser = args
            .open
            .unwrap_or_else(|| workspace.settings.always_open_browser.unwrap_or_default());

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

        let BuildTargets { client, server } = args.targets.into_targets().await?;

        // All servers will end up behind us (the devserver) but on a different port
        // This is so we can serve a loading screen as well as devtools without anything particularly fancy
        let fullstack = server.is_some();
        let should_proxy_port = match client.platform {
            Platform::Server => true,
            _ => fullstack,
        };

        let proxied_port = should_proxy_port
            .then(|| get_available_port(devserver_bind_ip, None))
            .flatten();

        let watch_fs = args.watch.unwrap_or(true);
        let use_hotpatch_engine = args.hot_patch;
        let build_mode = match use_hotpatch_engine {
            true => BuildMode::Fat,
            false => BuildMode::Base,
        };

        let client = AppBuilder::start(&client, build_mode.clone())?;
        let server = server
            .map(|server| AppBuilder::start(&server, build_mode))
            .transpose()?;

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
            applied_hot_reload_message: Default::default(),
            automatic_rebuilds: true,
            watch_fs,
            use_hotpatch_engine,
            client,
            server,
            hot_reload,
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
            tw_watcher,
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

        Ok(runner)
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
                    id: BuildId::CLIENT,
                    update: client_update,
                }
            }

            Some(server_update) = server_wait => {
                ServeUpdate::BuilderUpdate {
                    id: BuildId::SERVER,
                    update: server_update,
                }
            }

            // Wait for the watcher to send us an event
            event = watcher_wait => {
                let mut changes: Vec<_> = event.into_iter().collect();

                // Dequeue in bulk if we can, we might've received a lot of events in one go
                while let Some(event) = self.watcher_rx.try_next().ok().flatten() {
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
            tracing::debug!(
                "Ignoring file change: client is not ready to receive hotreloads. Files: {:#?}",
                files
            );
            return;
        }

        // If we have any changes to the rust files, we need to update the file map
        let mut templates = vec![];

        // Prepare the hotreload message we need to send
        let mut assets = Vec::new();
        let mut needs_full_rebuild = false;

        // We attempt to hotreload rsx blocks without a full rebuild
        for path in files {
            // for various assets that might be linked in, we just try to hotreloading them forcefully
            // That is, unless they appear in an include! macro, in which case we need to a full rebuild....
            let Some(ext) = path.extension().and_then(|v| v.to_str()) else {
                continue;
            };

            // If it's an asset, we want to hotreload it
            // todo(jon): don't hardcode this here
            if let Some(bundled_name) = self.client.hotreload_bundled_asset(path).await {
                assets.push(PathBuf::from("/assets/").join(bundled_name));
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
                    tracing::debug!("Filemap: {:#?}", self.file_map.keys());
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
                    needs_full_rebuild = true;
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
                        needs_full_rebuild = true;
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
        }

        // todo - we need to distinguish between hotpatchable rebuilds and true full rebuilds.
        //        A full rebuild is required when the user modifies static initializers which we haven't wired up yet.
        if needs_full_rebuild {
            if self.use_hotpatch_engine {
                self.client.patch_rebuild(files.to_vec());
                if let Some(server) = self.server.as_mut() {
                    server.patch_rebuild(files.to_vec());
                }
                self.clear_hot_reload_changes();
                self.clear_cached_rsx();
                server.send_patch_start().await;
            } else {
                self.client.start_rebuild(BuildMode::Base);
                if let Some(server) = self.server.as_mut() {
                    server.start_rebuild(BuildMode::Base);
                }
                self.clear_hot_reload_changes();
                self.clear_cached_rsx();
                server.send_reload_start().await;
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
            let file =
                file.trim_start_matches(&self.client.build.crate_dir().display().to_string());

            // Only send a hotreload message for templates and assets - otherwise we'll just get a full rebuild
            //
            // todo: move the android file uploading out of hotreload_bundled_asset and
            //
            // Also make sure the builder isn't busy since that might cause issues with hotreloads
            // https://github.com/DioxusLabs/dioxus/issues/3361
            if !msg.is_empty() && self.client.can_receive_hotreloads() {
                use crate::styles::NOTE_STYLE;
                tracing::info!(dx_src = ?TraceSrc::Dev, "Hotreloading: {NOTE_STYLE}{}{NOTE_STYLE:#}", file);

                if !server.has_hotreload_sockets() && self.client.build.platform != Platform::Web {
                    tracing::warn!("No clients to hotreload - try reloading the app!");
                }

                server.send_hotreload(msg).await;
            } else {
                tracing::debug!(dx_src = ?TraceSrc::Dev, "Ignoring file change: {}", file);
            }
        }
    }

    /// Finally "bundle" this app and return a handle to it
    pub(crate) async fn open(
        &mut self,
        artifacts: BuildArtifacts,
        devserver: &mut WebServer,
    ) -> Result<()> {
        // Make sure to save artifacts regardless of if we're opening the app or not
        match artifacts.platform {
            Platform::Server => {
                if let Some(server) = self.server.as_mut() {
                    server.artifacts = Some(artifacts.clone());
                }
            }
            _ => self.client.artifacts = Some(artifacts.clone()),
        }

        let should_open = self.client.stage == BuildStage::Success
            && (self.server.as_ref().map(|s| s.stage == BuildStage::Success)).unwrap_or(true);

        use crate::cli::styles::GLOW_STYLE;

        if should_open {
            let time_taken = artifacts
                .time_end
                .duration_since(artifacts.time_start)
                .unwrap();

            if self.client.builds_opened == 0 {
                tracing::info!(
                    "Build completed successfully in {GLOW_STYLE}{:?}ms{GLOW_STYLE:#}, launching app! 💫",
                    time_taken.as_millis()
                );
            } else {
                tracing::info!(
                    "Build completed in {GLOW_STYLE}{:?}ms{GLOW_STYLE:#}",
                    time_taken.as_millis()
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
        if let Some(server) = self.server.as_mut() {
            tracing::debug!("Opening server build");
            server.soft_kill().await;
            server
                .open(
                    devserver_ip,
                    displayed_address,
                    fullstack_address,
                    false,
                    false,
                    BuildId::SERVER,
                )
                .await?;
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
                BuildId::CLIENT,
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
        if matches!(self.client.build.platform, Platform::Android) {
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
        let build_mode = match self.use_hotpatch_engine {
            true => BuildMode::Fat,
            false => BuildMode::Base,
        };

        self.client.start_rebuild(build_mode.clone());
        if let Some(s) = self.server.as_mut() {
            s.start_rebuild(build_mode)
        }

        self.clear_hot_reload_changes();
        self.clear_cached_rsx();
        self.clear_patches();
    }

    pub(crate) async fn hotpatch(
        &mut self,
        res: &BuildArtifacts,
        id: BuildId,
        cache: &HotpatchModuleCache,
    ) -> Result<JumpTable> {
        let jump_table = match id {
            BuildId::CLIENT => self.client.hotpatch(res, cache).await,
            BuildId::SERVER => {
                self.server
                    .as_mut()
                    .context("Server not found")?
                    .hotpatch(res, cache)
                    .await
            }
            _ => return Err(Error::Runtime("Invalid build id".into())),
        }?;

        if id == BuildId::CLIENT {
            self.applied_hot_reload_message.jump_table = self.client.patches.last().cloned();
        }

        Ok(jump_table)
    }

    pub(crate) fn get_build(&self, id: BuildId) -> Option<&AppBuilder> {
        match id {
            BuildId::CLIENT => Some(&self.client),
            BuildId::SERVER => self.server.as_ref(),
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
        let mut msg = self.applied_hot_reload_message.clone();

        if build == BuildId::CLIENT {
            msg.jump_table = self.client.patches.last().cloned();
            msg.for_build_id = Some(BuildId::CLIENT.0 as _);
            if let Some(lib) = msg.jump_table.as_mut() {
                lib.lib = PathBuf::from("/").join(lib.lib.clone());
            }
        }

        if build == BuildId::SERVER {
            if let Some(server) = self.server.as_mut() {
                msg.jump_table = server.patches.last().cloned();
                msg.for_build_id = Some(BuildId::SERVER.0 as _);
            }
        }

        msg
    }

    /// Clear the hot reload changes. This should be called any time a new build is starting
    pub(crate) fn clear_hot_reload_changes(&mut self) {
        self.applied_hot_reload_message = Default::default();
    }

    pub(crate) fn clear_patches(&mut self) {
        self.client.patches.clear();
        if let Some(server) = self.server.as_mut() {
            server.patches.clear();
        }
    }

    pub(crate) async fn client_connected(
        &mut self,
        build_id: BuildId,
        aslr_reference: Option<u64>,
        pid: Option<u32>,
    ) {
        match build_id {
            BuildId::CLIENT => {
                // multiple tabs on web can cause this to be called incorrectly, and it doesn't
                // make any sense anyways
                if self.client.build.platform != Platform::Web {
                    if let Some(aslr_reference) = aslr_reference {
                        self.client.aslr_reference = Some(aslr_reference);
                    }
                    if let Some(pid) = pid {
                        self.client.pid = Some(pid);
                    }
                }
            }
            BuildId::SERVER => {
                if let Some(server) = self.server.as_mut() {
                    server.aslr_reference = aslr_reference;
                }
            }
            _ => {}
        }

        // Assign the runtime asset dir to the runner
        if self.client.build.platform == Platform::Ios {
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
        let applied = &mut self.applied_hot_reload_message;

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
    /// https://doc.rust-lang.org/stable/nightly-rustc/cargo/core/compiler/fingerprint/index.html#dep-info-files
    fn load_rsx_filemap(&mut self) {
        self.fill_filemap_from_krate(self.client.build.crate_dir());

        if let Some(server) = self.server.as_ref() {
            self.fill_filemap_from_krate(server.build.crate_dir());
        }

        for krate in self.all_watched_crates() {
            self.fill_filemap_from_krate(krate);
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
        for entry in walkdir::WalkDir::new(crate_dir).into_iter().flatten() {
            if self
                .workspace
                .ignore
                .matched(entry.path(), entry.file_type().is_dir())
                .is_ignore()
            {
                continue;
            }

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(contents) = std::fs::read_to_string(path) {
                    self.file_map.insert(
                        path.to_path_buf(),
                        CachedFile {
                            contents,
                            most_recent: None,
                            templates: Default::default(),
                        },
                    );
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
            tracing::trace!("Watching path {path:?}");

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
            tracing::trace!("Watching path {krate:?}");
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

    /// Get all the Manifest paths for dependencies that we should watch. Will not return anything
    /// in the `.cargo` folder - only local dependencies will be watched.
    ///
    /// This returns a list of manifest paths
    ///
    /// Extend the watch path to include:
    ///
    /// - the assets directory - this is so we can hotreload CSS and other assets by default
    /// - the Cargo.toml file - this is so we can hotreload the project if the user changes dependencies
    /// - the Dioxus.toml file - this is so we can hotreload the project if the user changes the Dioxus config
    fn local_dependencies(&self, crate_package: NodeId) -> Vec<PathBuf> {
        let mut paths = vec![];

        for (dependency, _edge) in self.workspace.krates.get_deps(crate_package) {
            let krate = match dependency {
                krates::Node::Krate { krate, .. } => krate,
                krates::Node::Feature { krate_index, .. } => {
                    &self.workspace.krates[krate_index.index()]
                }
            };

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
        }

        paths
    }

    // todo: we need to make sure we merge this for all the running packages
    fn all_watched_crates(&self) -> Vec<PathBuf> {
        let crate_package = self.client().build.crate_package;
        let crate_dir = self.client().build.crate_dir();

        let mut krates: Vec<PathBuf> = self
            .local_dependencies(crate_package)
            .into_iter()
            .map(|p| {
                p.parent()
                    .expect("Local manifest to exist and have a parent")
                    .to_path_buf()
            })
            .chain(Some(crate_dir))
            .collect();

        if let Some(server) = self.server.as_ref() {
            let server_crate_package = server.build.crate_package;
            let server_crate_dir = server.build.crate_dir();

            let server_krates: Vec<PathBuf> = self
                .local_dependencies(server_crate_package)
                .into_iter()
                .map(|p| {
                    p.parent()
                        .expect("Server manifest to exist and have a parent")
                        .to_path_buf()
                })
                .chain(Some(server_crate_dir))
                .collect();
            krates.extend(server_krates);
        }

        krates.dedup();

        krates
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
        if self.use_hotpatch_engine {
            tracing::warn!("Debugging symbols might not work properly with hotpatching enabled. Consider disabling hotpatching for debugging.");
        }

        match build {
            BuildId::CLIENT => {
                _ = self.client.open_debugger(dev).await;
            }
            BuildId::SERVER => {
                if let Some(server) = self.server.as_mut() {
                    _ = server.open_debugger(dev).await;
                }
            }
            _ => {}
        }
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
/// We determine this based on whether the keyword `microsoft` or `wsl` is contained within the [`WSL_1`] or [`WSL_2`] files.
/// This may fail in the future as it isn't guaranteed by Microsoft.
/// See https://github.com/microsoft/WSL/issues/423#issuecomment-221627364
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
