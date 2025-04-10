use super::{AppBuilder, ServeUpdate, WebServer, SELF_IP};
use crate::{
    AddressArguments, BuildArtifacts, BuildId, BuildMode, BuildRequest, Platform, Result,
    ServeArgs, TraceSrc, Workspace,
};
use anyhow::Context;
use axum::extract::ws::Message as WsMessage;
use dioxus_core::internal::{
    HotReloadTemplateWithLocation, HotReloadedTemplate, TemplateGlobalKey,
};
use dioxus_core_types::HotReloadingContext;
use dioxus_devtools_types::ClientMsg;
use dioxus_devtools_types::HotReloadMsg;
use dioxus_html::HtmlCtx;
use dioxus_rsx::CallBody;
use dioxus_rsx_hotreload::{ChangedRsx, HotReloadResult};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use futures_util::{future::OptionFuture, pin_mut};
use ignore::gitignore::Gitignore;
use krates::NodeId;
use notify::{
    event::{MetadataKind, ModifyKind},
    Config, EventKind, RecursiveMode, Watcher as NotifyWatcher,
};
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use std::{path::Path, time::SystemTime};
use subsecond_cli_support::JumpTable;
use syn::spanned::Spanned;
use target_lexicon::Triple;
use tokio::process::Command;

/// This is the primary "state" object that holds the builds and handles for the running apps.
///
/// It holds the resolved state from the ServeArgs, providing a source of truth for the rest of the app
///
/// It also holds the watcher which is used to watch for changes in the filesystem and trigger rebuilds,
/// hotreloads, asset updates, etc.
pub(crate) struct AppRunner {
    /// the platform of the "primary" crate (ie the first)
    pub(crate) workspace: Arc<Workspace>,

    pub(crate) client: AppBuilder,
    pub(crate) server: Option<AppBuilder>,

    // Related to to the filesystem watcher
    pub(crate) watcher: Box<dyn notify::Watcher>,
    pub(crate) watcher_tx: UnboundedSender<notify::Event>,
    pub(crate) watcher_rx: UnboundedReceiver<notify::Event>,

    // Tracked state related to open builds and hot reloading
    pub(crate) applied_hot_reload_message: HotReloadMsg,
    pub(crate) builds_opened: usize,
    pub(crate) file_map: HashMap<PathBuf, CachedFile>,

    // Resolved args related to how we go about processing the rebuilds and logging
    pub(crate) automatic_rebuilds: bool,
    pub(crate) interactive: bool,
    pub(crate) force_sequential: bool,
    pub(crate) hot_reload: bool,
    pub(crate) open_browser: bool,
    pub(crate) wsl_file_poll_interval: u16,
    pub(crate) always_on_top: bool,
    pub(crate) fullstack: bool,

    // resolve args related to the webserver
    pub(crate) devserver_port: u16,
    pub(crate) devserver_bind_ip: IpAddr,
    pub(crate) proxied_port: Option<u16>,
    pub(crate) cross_origin_policy: bool,
}

pub enum HotReloadKind {
    Rsx(HotReloadMsg),
    Patch,
    Full,
}

pub(crate) struct CachedFile {
    contents: String,
    most_recent: Option<String>,
    templates: HashMap<TemplateGlobalKey, HotReloadedTemplate>,
}

impl AppRunner {
    /// Create the AppRunner and then initialize the filemap with the crate directory.
    pub(crate) async fn start(args: ServeArgs) -> Result<Self> {
        let workspace = Workspace::current().await?;

        // Resolve the simpler args
        let interactive = args.is_interactive_tty();
        let force_sequential = args.build_arguments.force_sequential;
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

        // Use 0.0.0.0 as the default address if none is specified - this will let us expose the
        // devserver to the network (for other devices like phones/embedded)
        let devserver_bind_ip = args.address.addr.unwrap_or(SELF_IP);

        // If the user specified a port, use that, otherwise use any available port, preferring 8080
        let devserver_port = args
            .address
            .port
            .unwrap_or_else(|| get_available_port(devserver_bind_ip, Some(8080)).unwrap_or(8080));

        // Spin up the file watcher
        let (watcher_tx, watcher_rx) = futures_channel::mpsc::unbounded();
        let watcher = create_notify_watcher(watcher_tx.clone(), wsl_file_poll_interval as u64);

        // Now resolve the builds that we need to.
        // These come from the args, but we'd like them to come from the `TargetCmd` chained object
        //
        // The process here is as follows:
        //
        // - Create the BuildRequest for the primary target
        // - If that BuildRequest is "fullstack", then add the client features
        // - If that BuildRequest is "fullstack", then also create a BuildRequest for the server
        //   with the server features
        //
        // This involves modifying the BuildRequest to add the client features and server features
        // only if we can properly detect that it's a fullstack build. Careful with this, since
        // we didn't build BuildRequest to be generally mutable.
        let mut client = BuildRequest::new(&args.build_arguments).await?;
        let mut server = None;

        // Now we need to resolve the client features
        let fullstack = client.fullstack_feature_enabled() || args.fullstack.unwrap_or(false);
        if fullstack {
            let _server = BuildRequest::new(&args.build_arguments).await?;
            // ... todo: add the server features to the server build
            // ... todo: add the client features to the client build
            // // Make sure we have a server feature if we're building a fullstack app
            // if self.fullstack && self.server_features.is_empty() {
            //     return Err(anyhow::anyhow!("Fullstack builds require a server feature on the target crate. Add a `server` feature to the crate and try again.").into());
            // }

            // // Make sure we set the fullstack platform so we actually build the fullstack variant
            // // Users need to enable "fullstack" in their default feature set.
            // // todo(jon): fullstack *could* be a feature of the app, but right now we're assuming it's always enabled
            // let fullstack = args.fullstack || krate.has_dioxus_feature("fullstack");
            server = Some(_server);
        }

        // All servers will end up behind us (the devserver) but on a different port
        // This is so we can serve a loading screen as well as devtools without anything particularly fancy
        let should_proxy_port = match client.platform {
            Platform::Server => true,
            _ => fullstack,
            // During SSG, just serve the static files instead of running the server
            // _ => builds[0].fullstack && !self.build_arguments.ssg,
        };

        let proxied_port = should_proxy_port
            .then(|| get_available_port(devserver_bind_ip, None))
            .flatten();

        let client = AppBuilder::start(&client).unwrap();
        let server = server.map(|server| AppBuilder::start(&server).unwrap());

        // Create the runner
        let mut runner = Self {
            file_map: Default::default(),
            applied_hot_reload_message: Default::default(),
            builds_opened: 0,
            automatic_rebuilds: true,
            client,
            server,
            hot_reload,
            open_browser,
            wsl_file_poll_interval,
            always_on_top,
            workspace,
            devserver_port,
            devserver_bind_ip,
            proxied_port,
            watcher,
            watcher_rx,
            watcher_tx,
            interactive,
            force_sequential,
            cross_origin_policy,
            fullstack,
        };

        // Spin up the notify watcher
        // When builds load though, we're going to parse their depinfo and add the paths to the watcher
        runner.watch_filesystem();

        // todo(jon): this might take a while so we should try and background it, or make it lazy somehow
        // we could spawn a thread to search the FS and then when it returns we can fill the filemap
        // in testing, if this hits a massive directory, it might take several seconds with no feedback.
        // really, we should be using depinfo to get the files that are actually used, but the depinfo file might not be around yet
        // todo(jon): see if we can just guess the depinfo file before it generates. might be stale but at least it catches most of the files
        runner.load_rsx_filemap();

        Ok(runner)
    }

    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        let client = &mut self.client;

        let client_wait = client.wait();
        let watcher_wait = self.watcher_rx.next();

        // // If there are no running apps, we can just return pending to avoid deadlocking
        // let Some(handle) = self.running.as_mut() else {
        //     return futures_util::future::pending().await;
        // };

        tokio::select! {
            // Wait for the client to finish
            client_update = client_wait => {
                ServeUpdate::BuilderUpdate {
                    id: BuildId(0),
                    update: client_update,
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

                tracing::debug!("Files changed: {files:?}");

                ServeUpdate::FilesChanged { files }
            }
        }
    }

    pub(crate) fn rebuild_all(&mut self) {
        self.client.rebuild()
    }

    /// Finally "bundle" this app and return a handle to it
    pub(crate) async fn open(
        &mut self,
        app: BuildArtifacts,
        devserver_ip: SocketAddr,
        fullstack_address: Option<SocketAddr>,
    ) -> Result<()> {
        // Drop the old handle
        // This is a more forceful kill than soft_kill since the app entropy will be wiped
        self.cleanup().await;

        // Add some cute logging
        let time_taken = app.time_end.duration_since(app.time_start).unwrap();
        if self.builds_opened == 0 {
            tracing::info!(
                "Build completed successfully in {:?}ms, launching app! 💫",
                time_taken.as_millis()
            );
        } else {
            tracing::info!("Build completed in {:?}ms", time_taken.as_millis());
        }

        // Start the new app before we kill the old one to give it a little bit of time
        let open_browser = self.builds_opened == 0 && self.open_browser;
        let always_on_top = self.always_on_top;
        self.client
            .open(devserver_ip, fullstack_address, open_browser, always_on_top)
            .await?;
        self.builds_opened += 1;

        // Save the artifacts and clear the patches(?)
        self.client.artifacts = Some(app);

        Ok(())
    }

    /// Open an existing app bundle, if it exists
    pub(crate) async fn open_existing(&mut self, devserver: &WebServer) -> Result<()> {
        let fullstack_address = devserver.proxied_server_address();

        todo!();
        // if let Some(runner) = self.running.as_mut() {
        //     runner.soft_kill().await;
        //     runner
        //         .open(devserver.devserver_address(), fullstack_address, true)
        //         .await?;
        // }

        Ok(())
    }

    /// Shutdown all the running processes
    pub(crate) async fn cleanup(&mut self) {
        self.client.cleanup().await;

        if let Some(server) = self.server.as_mut() {
            server.cleanup().await;
        }

        // If the client is running on Android, we need to remove the port forwarding
        // todo: use the android tools "adb"
        if matches!(self.client.build.platform, Platform::Android) {
            use std::process::{Command, Stdio};
            if let Err(err) = Command::new("adb")
                .arg("reverse")
                .arg("--remove")
                .arg(format!("tcp:{}", self.devserver_port))
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .output()
            {
                tracing::error!(
                    "failed to remove forwarded port {}: {err}",
                    self.devserver_port
                );
            }
        }
    }

    // /// Attempt to hotreload the given files
    // pub(crate) async fn hotreload(&mut self, modified_files: Vec<PathBuf>) -> HotReloadKind {
    //     // If we have any changes to the rust files, we need to update the file map
    //     let mut templates = vec![];

    //     // Prepare the hotreload message we need to send
    //     let mut assets = Vec::new();
    //     let mut needs_full_rebuild = false;

    //     // We attempt to hotreload rsx blocks without a full rebuild
    //     for path in modified_files {
    //         // for various assets that might be linked in, we just try to hotreloading them forcefully
    //         // That is, unless they appear in an include! macro, in which case we need to a full rebuild....
    //         let Some(ext) = path.extension().and_then(|v| v.to_str()) else {
    //             continue;
    //         };

    //         // If it's a rust file, we want to hotreload it using the filemap
    //         if ext == "rs" {
    //             // Strip the prefix before sending it to the filemap
    //             if path.strip_prefix(self.krate.workspace_dir()).is_err() {
    //                 tracing::error!(
    //                     "Hotreloading file outside of the crate directory: {:?}",
    //                     path
    //                 );
    //                 continue;
    //             };

    //             // And grabout the contents
    //             let Ok(contents) = std::fs::read_to_string(&path) else {
    //                 tracing::debug!("Failed to read rust file while hotreloading: {:?}", path);
    //                 continue;
    //             };

    //             match self.rsx_changed::<HtmlCtx>(&path, contents) {
    //                 Some(new) => templates.extend(new),
    //                 None => needs_full_rebuild = true,
    //             }

    //             continue;
    //         }

    //         // Otherwise, it might be an asset and we should look for it in all the running apps
    //         if let Some(runner) = self.running.as_mut() {
    //             if let Some(bundled_name) = runner.hotreload_bundled_asset(&path).await {
    //                 // todo(jon): don't hardcode this here
    //                 assets.push(PathBuf::from("/assets/").join(bundled_name));
    //             }
    //         }
    //     }

    //     match needs_full_rebuild {
    //         true => HotReloadKind::Patch,
    //         false => {
    //             let msg = HotReloadMsg {
    //                 templates,
    //                 assets,
    //                 ..Default::default()
    //             };

    //             self.add_hot_reload_message(&msg);

    //             HotReloadKind::Rsx(msg)
    //         }
    //     }
    // }

    pub(crate) fn get_build(&self, id: BuildId) -> Option<&AppBuilder> {
        match id.0 {
            0 => Some(&self.client),
            1 => self.server.as_ref(),
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
    pub(crate) fn applied_hot_reload_changes(&mut self) -> HotReloadMsg {
        self.applied_hot_reload_message.clone()
    }

    /// Clear the hot reload changes. This should be called any time a new build is starting
    pub(crate) fn clear_hot_reload_changes(&mut self) {
        self.applied_hot_reload_message = Default::default();
    }

    pub(crate) async fn client_connected(&mut self) {
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
                .matched(&entry.path(), entry.file_type().is_dir())
                .is_ignore()
            {
                continue;
            }

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(path) = path.strip_prefix(self.workspace.workspace_dir()) {
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
    }

    /// Try to update the rsx in a file, returning the templates that were hotreloaded
    ///
    /// If the templates could not be hotreloaded, this will return an error. This error isn't fatal, per se,
    /// but it does mean that we could not successfully hotreload the file in-place.
    ///
    /// It's expected that the file path you pass in is relative the crate root. We have no way of
    /// knowing if it's *not*, so we'll assume it is.
    ///
    /// This does not do any caching on what intermediate state, like previous hotreloads, so you need
    /// to do that yourself.
    pub(crate) fn rsx_changed<Ctx: HotReloadingContext>(
        &mut self,
        path: &Path,
        new_contents: String,
    ) -> Option<Vec<HotReloadTemplateWithLocation>> {
        // Get the cached file if it exists - ignoring if it doesn't exist
        let Some(cached_file) = self.file_map.get_mut(path) else {
            tracing::debug!("No entry for file in filemap: {:?}", path);
            return Some(vec![]);
        };

        // We assume we can parse the old file and the new file
        // We should just ignore hotreloading files that we can't parse
        // todo(jon): we could probably keep the old `File` around instead of re-parsing on every hotreload
        let (Ok(old_file), Ok(new_file)) = (
            syn::parse_file(&cached_file.contents),
            syn::parse_file(&new_contents),
        ) else {
            tracing::debug!("Diff rsx returned not parseable");
            return Some(vec![]);
        };

        // todo(jon): allow server-fn hotreloading
        let Some(changed_rsx) = dioxus_rsx_hotreload::diff_rsx(&new_file, &old_file) else {
            return None;
        };

        // Update the most recent version of the file, so when we force a rebuild, we keep operating on the most recent version
        cached_file.most_recent = Some(new_contents);

        let mut out_templates = vec![];
        for ChangedRsx { old, new } in changed_rsx {
            let old_start = old.span().start();

            let old_parsed = syn::parse2::<CallBody>(old.tokens);
            let new_parsed = syn::parse2::<CallBody>(new.tokens);
            let (Ok(old_call_body), Ok(new_call_body)) = (old_parsed, new_parsed) else {
                continue;
            };

            // Format the template location, normalizing the path
            let file_name: String = path
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");

            // Returns a list of templates that are hotreloadable
            let results = HotReloadResult::new::<Ctx>(
                &old_call_body.body,
                &new_call_body.body,
                file_name.clone(),
            );

            // If no result is returned, we can't hotreload this file and need to keep the old file
            let Some(results) = results else {
                return None;
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
                out_templates.push(HotReloadTemplateWithLocation { template, key });
            }
        }

        Some(out_templates)
    }

    /// Commit the changes to the filemap, overwriting the contents of the files
    ///
    /// Removes any cached templates and replaces the contents of the files with the most recent
    ///
    /// todo: we should-reparse the contents so we never send a new version, ever
    pub(crate) fn clear_cached_rsx(&mut self) {
        for cached_file in self.file_map.values_mut() {
            if let Some(most_recent) = cached_file.most_recent.take() {
                cached_file.contents = most_recent;
            }
            cached_file.templates.clear();
        }
    }

    pub(crate) async fn patch(&mut self, res: &BuildArtifacts) -> Result<JumpTable> {
        let client = &self.client;
        let original = client.build.main_exe();
        let new = client.build.patch_exe(res.time_start);
        let triple = client.build.triple.clone();

        tracing::debug!("Patching {} -> {}", original.display(), new.display());

        let mut jump_table =
            subsecond_cli_support::create_jump_table(&original, &new, &triple).unwrap();

        tracing::debug!("Jump table: {:#?}", jump_table);

        // If it's android, we need to copy the assets to the device and then change the location of the patch
        if client.build.platform == Platform::Android {
            jump_table.lib = client
                .copy_file_to_android_tmp(&new, &(PathBuf::from(new.file_name().unwrap())))
                .await?;
        }

        // Rebase the wasm binary to be relocatable once the jump table is generated
        if triple.architecture == target_lexicon::Architecture::Wasm32 {
            let old_bytes = std::fs::read(&original).unwrap();
            let new_bytes = std::fs::read(&jump_table.lib).unwrap();
            let res_ = subsecond_cli_support::satisfy_got_imports(&old_bytes, &new_bytes).unwrap();
            std::fs::write(&jump_table.lib, res_).unwrap();

            // make sure we use the dir relative to the public dir
            let public_dir = client.build.root_dir();
            jump_table.lib = jump_table
                .lib
                .strip_prefix(&public_dir)
                .unwrap()
                .to_path_buf();
        }

        let changed_files = match &res.mode {
            BuildMode::Thin { changed_files, .. } => changed_files.clone(),
            _ => vec![],
        };

        let changed_file = changed_files.first().unwrap();
        tracing::info!(
            "Hot-patching: {} in {:?}ms",
            changed_file
                .strip_prefix(std::env::current_dir().unwrap())
                .unwrap_or_else(|_| changed_file.as_path())
                .display(),
            SystemTime::now()
                .duration_since(res.time_start)
                .unwrap()
                .as_millis()
        );

        // Save this patch
        self.client.patches.push(jump_table.clone());

        tracing::info!("jump table: {:#?}", jump_table);

        Ok(jump_table)
    }

    /// Handles incoming WebSocket messages from the client.
    ///
    /// This function processes messages sent by the client over the WebSocket connection. We only
    /// handle text messages, and we expect them to be in JSON format.
    ///
    /// Specifically, it handles the initialization message to set the Address Space Layout Randomization (ASLR) reference offset.
    ///
    /// For WebAssembly (Wasm) targets, ASLR is not used, so this value is ignored.
    pub(crate) async fn handle_ws_message(&mut self, msg: &WsMessage) -> Result<()> {
        let as_text = msg
            .to_text()
            .context("client message not proper encoding")?;

        match serde_json::from_str::<ClientMsg>(as_text) {
            Ok(ClientMsg::Initialize { aslr_reference }) => {
                tracing::debug!("Setting aslr_reference: {aslr_reference}");
                self.client.aslr_reference = Some(aslr_reference);
            }
            Ok(_client) => {}
            Err(err) => {
                tracing::error!(dx_src = ?TraceSrc::Dev, "Error parsing message from {}: {}", Platform::Web, err);
            }
        };

        Ok(())
    }

    fn watch_filesystem(&mut self) {
        // Watch the folders of the crates that we're interested in
        for path in self.watch_paths(
            self.client.build.crate_dir(),
            self.client.build.crate_package,
        ) {
            tracing::debug!("Watching path {path:?}");

            if let Err(err) = self.watcher.watch(&path, RecursiveMode::Recursive) {
                handle_notify_error(err);
            }
        }

        // Also watch the crates themselves, but not recursively, such that we can pick up new folders
        for krate in self.all_watched_crates() {
            tracing::debug!("Watching path {krate:?}");
            if let Err(err) = self.watcher.watch(&krate, RecursiveMode::NonRecursive) {
                handle_notify_error(err);
            }
        }

        // Also watch the workspace dir, non recursively, such that we can pick up new folders there too
        if let Err(err) = self.watcher.watch(
            &self.workspace.krates.workspace_root().as_std_path(),
            RecursiveMode::NonRecursive,
        ) {
            handle_notify_error(err);
        }
    }

    /// Return the list of paths that we should watch for changes.
    pub(crate) fn watch_paths(&self, crate_dir: PathBuf, crate_package: NodeId) -> Vec<PathBuf> {
        let mut watched_paths = vec![];

        // Get a list of *all* the crates with Rust code that we need to watch.
        // This will end up being dependencies in the workspace and non-workspace dependencies on the user's computer.
        let mut watched_crates = self.local_dependencies(crate_package);
        watched_crates.push(crate_dir);

        // Now, watch all the folders in the crates, but respecting their respective ignore files
        for krate_root in watched_crates {
            // Build the ignore builder for this crate, but with our default ignore list as well
            let ignore = self.workspace.ignore_for_krate(&krate_root);

            for entry in krate_root.read_dir().unwrap() {
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
    pub(crate) fn local_dependencies(&self, crate_package: NodeId) -> Vec<PathBuf> {
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
        .map(|listener| listener.local_addr().unwrap().port())
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
