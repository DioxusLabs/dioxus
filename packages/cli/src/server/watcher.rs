use std::{
    collections::HashSet,
    path::{PathBuf},
};

use dioxus_cli_config::CrateConfig;
use dioxus_hot_reload::HotReloadMsg;
use dioxus_html::HtmlCtx;
use dioxus_rsx::{hot_reload::FileMap};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use notify::{event::ModifyKind, EventKind, FsEventWatcher};

/// This struct stores the file watcher and the filemap for the project.
///
/// This is where we do workspace discovery and recursively listen for changes in Rust files and asset
/// directories.
pub struct Watcher {
    tx: UnboundedSender<notify::Event>,
    rx: UnboundedReceiver<notify::Event>,
    last_update_time: i64,
    watcher: FsEventWatcher,
    queued_events: Vec<notify::Event>,
    file_map: FileMap,
}

impl Watcher {
    pub fn start(config: &CrateConfig) -> Self {
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

            use notify::Watcher;
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

    pub fn attempt_hot_reload(&mut self, config: &CrateConfig) -> Option<HotReloadMsg> {
        let mut edited_rust_files = HashSet::new();
        let mut changed_assets = HashSet::new();

        let mut all_mods: Vec<(EventKind, PathBuf)> = vec![];

        // Decompose the events into a list of all the files that have changed
        for evt in self.queued_events.drain(..) {
            for modi in evt.paths {
                all_mods.push((evt.kind, modi.clone()));
            }
        }

        // For the non-rust files, we want to check if it's an asset file
        // This would mean the asset lives somewhere under the /assets directory or is referenced by magnanis in the linker
        // todo: mg integration here
        let asset_dir = config
            .dioxus_config
            .application
            .asset_dir
            .clone()
            .canonicalize()
            .expect("Asset dir to be valid");

        for (kind, path) in all_mods.iter() {
            // for various assets that might be linked in, we just try to hotreloading them forcefully
            // That is, unless they appear in an include! macro, in which case we need to a full rebuild....
            let ext = path.extension().and_then(|v| v.to_str())?;

            // Workaround for notify and vscode-like editor:
            // when edit & save a file in vscode, there will be two notifications,
            // the first one is a file with empty content.
            // filter the empty file notification to avoid false rebuild during hot-reload
            if let Ok(metadata) = std::fs::metadata(path) {
                if metadata.len() == 0 {
                    continue;
                }
            }

            // If the extension is a backup file, or a hidden file, ignore it completely (no rebuilds)
            if is_backup_file(path.to_path_buf()) {
                tracing::trace!("Ignoring backup file: {:?}", path);
                continue;
            }

            // todo: handle gitignore

            // Only handle .rs files that are changed since adds/removes don't necessarily change a rust project itself
            if ext == "rs" && kind == &EventKind::Modify(ModifyKind::Any) {
                edited_rust_files.insert(path);
            }

            if ext != "rs" && path.starts_with(&asset_dir) {
                changed_assets.insert(path);
            }
        }

        // If we have any changes to the rust files, we need to update the file map
        let crate_dir = config.crate_dir.clone();
        let mut changed_templates = vec![];

        for rust_file in edited_rust_files {
            let hotreloaded_templates = self
                .file_map
                .update_rsx::<HtmlCtx>(rust_file, &crate_dir)
                .ok()?;

            changed_templates.extend(hotreloaded_templates);
        }

        Some(HotReloadMsg {
            templates: changed_templates,
            assets: changed_assets.into_iter().cloned().collect(),
        })
    }

    /// Ensure the changes we've received from the queue are actually legit changes to either assets or
    /// rust code. We don't care about changes otherwise, unless we get a signle elsewhere to do a full rebuild
    pub fn pending_changes(&mut self) -> bool {
        !self.queued_events.is_empty()
    }
}

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

fn is_backup_file(path: PathBuf) -> bool {
    // If there's a tilde at the end of the file, it's a backup file
    if let Some(name) = path.file_name() {
        if let Some(name) = name.to_str() {
            if name.ends_with('~') {
                return true;
            }
        }
    }

    // if the file is hidden, it's a backup file
    if let Some(name) = path.file_name() {
        if let Some(name) = name.to_str() {
            if name.starts_with('.') {
                return true;
            }
        }
    }

    false
}

#[test]
fn test_is_backup_file() {
    assert!(is_backup_file(PathBuf::from("examples/test.rs~")));
    assert!(is_backup_file(PathBuf::from("examples/.back")));
    assert!(is_backup_file(PathBuf::from("test.rs~")));
    assert!(is_backup_file(PathBuf::from(".back")));

    assert!(!is_backup_file(PathBuf::from("val.rs")));
    assert!(!is_backup_file(PathBuf::from(
        "/Users/jonkelley/Development/Tinkering/basic_05_example/src/lib.rs"
    )));
    assert!(!is_backup_file(PathBuf::from("exmaples/val.rs")));
}
