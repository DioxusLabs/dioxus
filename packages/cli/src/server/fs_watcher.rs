use dioxus_html::HtmlCtx;
use dioxus_rsx::{hot_reload::FileMap, HotReload};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use notify::{FsEventWatcher, Watcher};

pub struct FileWatcher {
    tx: UnboundedSender<notify::Event>,
    rx: UnboundedReceiver<notify::Event>,
    last_update_time: i64,
    watcher: FsEventWatcher,
    queued_events: Vec<notify::Event>,
    file_map: FileMap,
}

impl FileWatcher {
    pub fn start(config: &dioxus_cli_config::CrateConfig) -> Self {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        // Extend the watch path to include:
        // - the assets directory - this is so we can hotreload CSS and other assets by default
        // - the Cargo.toml file - this is so we can hotreload the project if the user changes dependencies
        // - the Dioxus.toml file - this is so we can hotreload the project if the user changes the Dioxus config
        let mut allow_watch_path = config.dioxus_config.web.watcher.watch_path.clone();
        allow_watch_path.push(config.dioxus_config.application.asset_dir.clone());
        allow_watch_path.push("Cargo.toml".to_string().into());
        allow_watch_path.push("Dioxus.toml".to_string().into());
        allow_watch_path.dedup();

        // Create the file watcher
        let mut watcher = notify::recommended_watcher({
            let tx = tx.clone();
            move |info: notify::Result<notify::Event>| {
                if let Ok(e) = info {
                    _ = tx.unbounded_send(e);
                }
            }
        })
        .expect("Failed to create file watcher.\nEnsure you have the required permissions to watch the specified directories.");

        // Watch the specified paths
        // todo: make sure we don't double-watch paths if they're nested
        for sub_path in allow_watch_path {
            let path = &config.crate_dir.join(sub_path);
            let mode = notify::RecursiveMode::Recursive;

            if let Err(err) = watcher.watch(path, mode) {
                tracing::warn!("Failed to watch path: {}", err);
            }
        }

        // Probe the entire project looking for our rsx calls
        // Whenever we get an update from the file watcher, we'll try to hotreload against this file map
        let file_map = FileMap::create::<HtmlCtx>(config.crate_dir.clone()).unwrap();

        Self {
            tx,
            rx,
            watcher,
            file_map,
            queued_events: Vec::new(),
            last_update_time: chrono::Local::now().timestamp(),
        }
    }

    /// A cancel safe handle to the file watcher
    ///
    /// todo: this should be simpler logic?
    pub async fn wait(&mut self) {
        // Pull off any queued events in succession
        while let Ok(Some(event)) = self.rx.try_next() {
            self.queued_events.push(event);
        }

        if !self.queued_events.is_empty() {
            return;
        }

        // If there are no queued events, wait for the next event
        if let Some(event) = self.rx.next().await {
            self.queued_events.push(event);
        }
    }

    pub fn attempt_hot_reload(&mut self) -> Option<HotReload> {
        todo!()
    }

    pub fn attempt_binary_patch(&mut self) -> Option<Vec<u8>> {
        todo!("Attempt to binary patch the project")
    }
}

// fn watch_event<F>(
//     event: notify::Event,
//     last_update_time: &mut i64,
//     hot_reload: &HotReloadState,
//     config: &CrateConfig,
//     build_with: &F,
//     web_info: &Option<WebServerInfo>,
// ) where
//     F: Fn() -> Result<BuildResult> + Send + 'static,
// {
//     // Ensure that we're tracking only modifications
//     if !matches!(
//         event.kind,
//         notify::EventKind::Create(_) | notify::EventKind::Remove(_) | notify::EventKind::Modify(_)
//     ) {
//         return;
//     }

//     // Ensure that we're not rebuilding too frequently
//     if chrono::Local::now().timestamp() <= *last_update_time {
//         return;
//     }

//     // By default we want to not do a full rebuild, and instead let the hot reload system invalidate it
//     let mut needs_full_rebuild = false;

//     if let Some(file_map) = &hot_reload.file_map {
//         hotreload_files(
//             hot_reload,
//             file_map,
//             &mut needs_full_rebuild,
//             &event,
//             config,
//         );
//     }

//     if needs_full_rebuild {
//         full_rebuild(build_with, last_update_time, config, event, web_info);
//     }
// }

// fn full_rebuild<F>(
//     build_with: &F,
//     last_update_time: &mut i64,
//     config: &CrateConfig,
//     event: notify::Event,
//     web_info: &Option<WebServerInfo>,
// ) where
//     F: Fn() -> Result<BuildResult> + Send + 'static,
// {
//     match build_with() {
//         Ok(res) => {
//             *last_update_time = chrono::Local::now().timestamp();

//             #[allow(clippy::redundant_clone)]
//             print_console_info(
//                 config,
//                 PrettierOptions {
//                     changed: event.paths.clone(),
//                     warnings: res.warnings,
//                     elapsed_time: res.elapsed_time,
//                 },
//                 web_info.clone(),
//             );
//         }
//         Err(e) => {
//             *last_update_time = chrono::Local::now().timestamp();
//             tracing::error!("{:?}", e);
//         }
//     }
// }

// fn hotreload_files(
//     hot_reload: &HotReloadState,
//     file_map: &SharedFileMap,
//     needs_full_rebuild: &mut bool,
//     event: &notify::Event,
//     config: &CrateConfig,
// ) {
//     // find changes to the rsx in the file
//     let mut rsx_file_map = file_map.lock().unwrap();
//     let mut messages: Vec<HotReloadMsg> = Vec::new();

//     for path in &event.paths {
//         // Attempt to hotreload this file
//         let is_potentially_reloadable = hotreload_file(
//             path,
//             config,
//             &rsx_file_map,
//             &mut messages,
//             needs_full_rebuild,
//         );

//         // If the file was not hotreloaded, continue
//         if is_potentially_reloadable.is_none() {
//             continue;
//         }

//         // If the file was hotreloaded, update the file map in place
//         match rsx_file_map.update_rsx(path, &config.crate_dir) {
//             Ok(UpdateResult::UpdatedRsx {
//                 templates,
//                 changed_lits: changed_strings,
//             }) => {
//                 messages.push(HotReloadMsg::Update {
//                     templates,
//                     changed_strings,
//                     assets: vec![],
//                 });
//             }

//             // If the file was not updated, we need to do a full rebuild
//             Ok(UpdateResult::NeedsRebuild) => {
//                 tracing::trace!("Needs full rebuild because file changed: {:?}", path);
//                 *needs_full_rebuild = true;
//             }

//             // Not necessarily a fatal error, but we should log it
//             Err(err) => tracing::error!("{}", err),
//         }
//     }

//     // If full rebuild, extend the file map with the new file map
//     // This will wipe away any previous cached changed templates
//     if *needs_full_rebuild {
//         // Reset the file map to the new state of the project
//         let FileMapBuildResult {
//             map: new_file_map,
//             errors,
//         } = FileMap::<HtmlCtx>::create(config.crate_dir.clone()).unwrap();

//         for err in errors {
//             tracing::error!("{}", err);
//         }

//         *rsx_file_map = new_file_map;

//         return;
//     }

//     for msg in messages {
//         hot_reload.receiver.send_message(msg);
//     }
// }

// fn hotreload_file(
//     path: &Path,
//     config: &CrateConfig,
//     rsx_file_map: &std::sync::MutexGuard<'_, FileMap<HtmlCtx>>,
//     messages: &mut Vec<HotReloadMsg>,
//     needs_full_rebuild: &mut bool,
// ) -> Option<()> {
//     // for various assets that might be linked in, we just try to hotreloading them forcefully
//     // That is, unless they appear in an include! macro, in which case we need to a full rebuild....
//     let ext = path.extension().and_then(|v| v.to_str())?;

//     // Workaround for notify and vscode-like editor:
//     // when edit & save a file in vscode, there will be two notifications,
//     // the first one is a file with empty content.
//     // filter the empty file notification to avoid false rebuild during hot-reload
//     if let Ok(metadata) = fs::metadata(path) {
//         if metadata.len() == 0 {
//             return None;
//         }
//     }

//     // If the extension is a backup file, or a hidden file, ignore it completely (no rebuilds)
//     if is_backup_file(path) {
//         tracing::trace!("Ignoring backup file: {:?}", path);
//         return None;
//     }

//     // Attempt to hotreload css in the asset directory
//     // Currently no other assets are hotreloaded, but in theory we could hotreload pngs/jpegs, etc
//     //
//     // All potential hotreloadable mime types:
//     // "bin" |"css" | "csv" | "html" | "ico" | "js" | "json" | "jsonld" | "mjs" | "rtf" | "svg" | "mp4"
//     if ext == "css" {
//         let asset_dir = config
//             .crate_dir
//             .join(&config.dioxus_config.application.asset_dir);

//         // Only if the CSS is in the asset directory, and we're tracking it, do we hotreload it
//         // Otherwise, we need to do a full rebuild since the user might be doing an include_str! on it
//         if attempt_css_reload(path, asset_dir, rsx_file_map, config, messages).is_none() {
//             *needs_full_rebuild = true;
//         }

//         return None;
//     }

//     // If the file is not rsx or css and we've already not needed a full rebuild, return
//     if ext != "rs" && ext != "css" {
//         *needs_full_rebuild = true;
//         return None;
//     }

//     Some(())
// }

// fn attempt_css_reload(
//     path: &Path,
//     asset_dir: PathBuf,
//     rsx_file_map: &std::sync::MutexGuard<'_, FileMap<HtmlCtx>>,
//     config: &CrateConfig,
//     messages: &mut Vec<HotReloadMsg>,
// ) -> Option<()> {
//     // If the path is not in the asset directory, return
//     if !path.starts_with(asset_dir) {
//         return None;
//     }

//     // Get the local path of the asset (ie var.css or some_dir/var.css as long as the dir is under the asset dir)
//     let local_path = local_path_of_asset(path)?;

//     // Make sure we're actually tracking this asset...
//     _ = rsx_file_map.is_tracking_asset(&local_path)?;

//     // copy the asset over to the output directory
//     // todo this whole css hotreloading should be less hacky and more robust
//     _ = fs_extra::copy_items(
//         &[path],
//         config.out_dir(),
//         &CopyOptions::new().overwrite(true),
//     );

//     messages.push(HotReloadMsg::Update {
//         templates: Default::default(),
//         changed_strings: Default::default(),
//         assets: vec![local_path],
//     });

//     Some(())
// }

// fn local_path_of_asset(path: &Path) -> Option<PathBuf> {
//     path.file_name()?.to_str()?.to_string().parse().ok()
// }

// pub(crate) trait Platform {
//     fn start(
//         config: &CrateConfig,
//         serve: &ConfigOptsServe,
//         env: Vec<(String, String)>,
//     ) -> Result<Self>
//     where
//         Self: Sized;
//     fn rebuild(
//         &mut self,
//         config: &CrateConfig,
//         serve: &ConfigOptsServe,
//         env: Vec<(String, String)>,
//     ) -> Result<BuildResult>;
// }

// fn is_backup_file(path: &Path) -> bool {
//     // If there's a tilde at the end of the file, it's a backup file
//     if let Some(name) = path.file_name() {
//         if let Some(name) = name.to_str() {
//             if name.ends_with('~') {
//                 return true;
//             }
//         }
//     }

//     // if the file is hidden, it's a backup file
//     if let Some(name) = path.file_name() {
//         if let Some(name) = name.to_str() {
//             if name.starts_with('.') {
//                 return true;
//             }
//         }
//     }

//     false
// }

// #[test]
// fn test_is_backup_file() {
//     assert!(is_backup_file(&PathBuf::from("examples/test.rs~")));
//     assert!(is_backup_file(&PathBuf::from("examples/.back")));
//     assert!(is_backup_file(&PathBuf::from("test.rs~")));
//     assert!(is_backup_file(&PathBuf::from(".back")));

//     assert!(!is_backup_file(&PathBuf::from("val.rs")));
//     assert!(!is_backup_file(&PathBuf::from(
//         "/Users/jonkelley/Development/Tinkering/basic_05_example/src/lib.rs"
//     )));
//     assert!(!is_backup_file(&PathBuf::from("exmaples/val.rs")));
// }
