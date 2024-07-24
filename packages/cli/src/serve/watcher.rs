use std::path::PathBuf;

use crate::dioxus_crate::DioxusCrate;
use crate::serve::hot_reloading_file_map::FileMap;
use dioxus_hot_reload::HotReloadMsg;
use dioxus_html::HtmlCtx;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use ignore::gitignore::Gitignore;
use notify::{event::ModifyKind, EventKind, RecommendedWatcher};

/// This struct stores the file watcher and the filemap for the project.
///
/// This is where we do workspace discovery and recursively listen for changes in Rust files and asset
/// directories.
pub struct Watcher {
    _tx: UnboundedSender<notify::Event>,
    rx: UnboundedReceiver<notify::Event>,
    _last_update_time: i64,
    _watcher: RecommendedWatcher,
    queued_events: Vec<notify::Event>,
    file_map: FileMap,
    ignore: Gitignore,
}

impl Watcher {
    pub fn start(config: &DioxusCrate) -> Self {
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

        let crate_dir = config.crate_dir();
        let mut builder = ignore::gitignore::GitignoreBuilder::new(&crate_dir);
        builder.add(crate_dir.join(".gitignore"));

        let out_dir = config.out_dir();
        let out_dir_str = out_dir.display().to_string();

        let excluded_paths = vec![
            ".git",
            ".github",
            ".vscode",
            "target",
            "node_modules",
            "dist",
            ".dioxus",
            &out_dir_str,
        ];
        for path in excluded_paths {
            builder
                .add_line(None, path)
                .expect("failed to add path to file excluder");
        }
        let ignore = builder.build().unwrap();

        // Create the file watcher
        let mut watcher = notify::recommended_watcher({
            let tx = tx.clone();
            move |info: notify::Result<notify::Event>| {
                if let Ok(e) = info {
                     match e.kind {

                        // An event emitted when the metadata of a file or folder is changed.
                        EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Any) |
                        EventKind::Create(_) |
                        EventKind::Remove(_) => {
                            _ = tx.unbounded_send(e);
                        },
                        _ => {}
                    }


                }
            }
        })
        .expect("Failed to create file watcher.\nEnsure you have the required permissions to watch the specified directories.");

        // Watch the specified paths
        // todo: make sure we don't double-watch paths if they're nested
        for sub_path in allow_watch_path {
            let path = &config.crate_dir().join(sub_path);

            // If the path is ignored, don't watch it
            if ignore.matched(path, path.is_dir()).is_ignore() {
                continue;
            }

            let mode = notify::RecursiveMode::Recursive;

            use notify::Watcher;
            if let Err(err) = watcher.watch(path, mode) {
                tracing::warn!("Failed to watch path: {}", err);
            }
        }

        // Probe the entire project looking for our rsx calls
        // Whenever we get an update from the file watcher, we'll try to hotreload against this file map
        let file_map = FileMap::create_with_filter::<HtmlCtx>(config.crate_dir(), |path| {
            ignore.matched(path, path.is_dir()).is_ignore()
        })
        .unwrap();

        Self {
            _tx: tx,
            rx,
            _watcher: watcher,
            file_map,
            ignore,
            queued_events: Vec::new(),
            _last_update_time: chrono::Local::now().timestamp(),
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

    /// Deques changed files from the event queue, doing the proper intelligent filtering
    pub fn dequeue_changed_files(&mut self, config: &DioxusCrate) -> Vec<PathBuf> {
        let mut all_mods: Vec<PathBuf> = vec![];

        // Decompose the events into a list of all the files that have changed
        for event in self.queued_events.drain(..) {
            // We only care about modify/crate/delete events
            match event.kind {
                EventKind::Modify(ModifyKind::Any) => {}
                EventKind::Modify(ModifyKind::Data(_)) => {}
                EventKind::Modify(ModifyKind::Name(_)) => {}
                EventKind::Create(_) => {}
                EventKind::Remove(_) => {}
                _ => continue,
            }

            for path in event.paths {
                all_mods.push(path.clone());
            }
        }

        let mut modified_files = vec![];

        // For the non-rust files, we want to check if it's an asset file
        // This would mean the asset lives somewhere under the /assets directory or is referenced by magnanis in the linker
        // todo: mg integration here
        let _asset_dir = config
            .dioxus_config
            .application
            .asset_dir
            .canonicalize()
            .ok();

        for path in all_mods.iter() {
            if path.extension().is_none() {
                continue;
            }

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

            // If the path is ignored, don't watch it
            if self.ignore.matched(path, path.is_dir()).is_ignore() {
                continue;
            }

            modified_files.push(path.clone());
        }

        modified_files
    }

    pub fn attempt_hot_reload(
        &mut self,
        config: &DioxusCrate,
        modified_files: Vec<PathBuf>,
    ) -> Option<HotReloadMsg> {
        // If we have any changes to the rust files, we need to update the file map
        let crate_dir = config.crate_dir();
        let mut templates = vec![];

        // Prepare the hotreload message we need to send
        let mut edited_rust_files = Vec::new();
        let mut assets = Vec::new();
        let mut unknown_files = Vec::new();

        for path in modified_files {
            // for various assets that might be linked in, we just try to hotreloading them forcefully
            // That is, unless they appear in an include! macro, in which case we need to a full rebuild....
            let Some(ext) = path.extension().and_then(|v| v.to_str()) else {
                continue;
            };

            match ext {
                "rs" => edited_rust_files.push(path),
                _ if path.starts_with("assets") => assets.push(path),
                _ => unknown_files.push(path),
            }
        }

        for rust_file in edited_rust_files {
            let hotreloaded_templates = self
                .file_map
                .update_rsx::<HtmlCtx>(&rust_file, &crate_dir)
                .ok()?;

            templates.extend(hotreloaded_templates);
        }

        Some(HotReloadMsg {
            templates,
            assets,
            unknown_files,
        })
    }

    /// Ensure the changes we've received from the queue are actually legit changes to either assets or
    /// rust code. We don't care about changes otherwise, unless we get a signal elsewhere to do a full rebuild
    pub fn pending_changes(&mut self) -> bool {
        !self.queued_events.is_empty()
    }
}

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
