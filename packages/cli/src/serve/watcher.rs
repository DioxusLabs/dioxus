use super::{detect::is_wsl, AppRunner};
use super::{hot_reloading_file_map::HotreloadError, update::ServeUpdate};
use crate::serve::hot_reloading_file_map::FileMap;
use crate::{cli::serve::ServeArgs, dioxus_crate::DioxusCrate};
use dioxus_devtools_types::HotReloadMsg;
use dioxus_html::HtmlCtx;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use ignore::gitignore::Gitignore;
use notify::{
    event::{MetadataKind, ModifyKind},
    Config, EventKind,
};
use std::collections::{HashMap, HashSet};
use std::{path::PathBuf, time::Duration};

/// This struct stores the file watcher and the filemap for the project.
///
/// This is where we do workspace discovery and recursively listen for changes in Rust files and asset
/// directories.
pub(crate) struct Watcher {
    rx: UnboundedReceiver<notify::Event>,
    krate: DioxusCrate,
    file_map: FileMap,
    ignore: Gitignore,
    applied_hot_reload_message: Option<HotReloadMsg>,
    _tx: UnboundedSender<notify::Event>,
    _last_update_time: i64,
    _watcher: Box<dyn notify::Watcher>,
}

impl Watcher {
    pub(crate) fn start(serve: &ServeArgs, krate: &DioxusCrate) -> Self {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        // Extend the watch path to include:
        // - the assets directory - this is so we can hotreload CSS and other assets by default
        // - the Cargo.toml file - this is so we can hotreload the project if the user changes dependencies
        // - the Dioxus.toml file - this is so we can hotreload the project if the user changes the Dioxus config
        let mut allow_watch_path = krate.dioxus_config.web.watcher.watch_path.clone();
        allow_watch_path.push(krate.dioxus_config.application.asset_dir.clone());
        allow_watch_path.push("Cargo.toml".to_string().into());
        allow_watch_path.push("Dioxus.toml".to_string().into());
        allow_watch_path.push("assets".to_string().into());
        allow_watch_path.dedup();

        let crate_dir = krate.crate_dir();
        let mut builder = ignore::gitignore::GitignoreBuilder::new(&crate_dir);
        builder.add(crate_dir.join(".gitignore"));

        let out_dir = krate.out_dir();
        let out_dir_str = out_dir.display().to_string();

        let excluded_paths = vec![
            ".git",
            ".github",
            ".vscode",
            "target",
            "node_modules",
            "dist",
            &out_dir_str,
        ];
        for path in excluded_paths {
            builder
                .add_line(None, path)
                .expect("failed to add path to file excluder");
        }
        let ignore = builder.build().unwrap();

        // Build the event handler for notify.
        let notify_event_handler = {
            let tx = tx.clone();
            move |info: notify::Result<notify::Event>| {
                if let Ok(e) = info {
                    if is_allowed_notify_event(&e) {
                        _ = tx.unbounded_send(e);
                    }
                }
            }
        };

        // If we are in WSL, we must use Notify's poll watcher due to an event propagation issue.
        let is_wsl = is_wsl();
        const NOTIFY_ERROR_MSG: &str = "Failed to create file watcher.\nEnsure you have the required permissions to watch the specified directories.";

        // Create the file watcher.
        let mut watcher: Box<dyn notify::Watcher> = match is_wsl {
            true => {
                let poll_interval =
                    Duration::from_secs(serve.wsl_file_poll_interval.unwrap_or(2) as u64);

                Box::new(
                    notify::PollWatcher::new(
                        notify_event_handler,
                        Config::default().with_poll_interval(poll_interval),
                    )
                    .expect(NOTIFY_ERROR_MSG),
                )
            }
            false => {
                Box::new(notify::recommended_watcher(notify_event_handler).expect(NOTIFY_ERROR_MSG))
            }
        };

        // Watch the specified paths
        // todo: make sure we don't double-watch paths if they're nested
        for sub_path in allow_watch_path {
            let path = &krate.crate_dir().join(sub_path);

            // If the path is ignored, don't watch it
            if ignore.matched(path, path.is_dir()).is_ignore() {
                continue;
            }

            let mode = notify::RecursiveMode::Recursive;

            if let Err(err) = watcher.watch(path, mode) {
                tracing::warn!("Failed to watch path: {}", err);
            }
        }

        // Probe the entire project looking for our rsx calls
        // Whenever we get an update from the file watcher, we'll try to hotreload against this file map
        let file_map = FileMap::create_with_filter::<HtmlCtx>(krate.crate_dir(), |path| {
            ignore.matched(path, path.is_dir()).is_ignore()
        })
        .unwrap();

        Self {
            _tx: tx,
            krate: krate.clone(),
            rx,
            _watcher: watcher,
            file_map,
            ignore,
            _last_update_time: chrono::Local::now().timestamp(),
            applied_hot_reload_message: None,
        }
    }

    /// Wait for changed files to be detected
    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        // Wait for the next file to change
        let mut changes: Vec<_> = self.rx.next().await.into_iter().collect();

        // Dequeue in bulk if we can, we might've received a lot of events in one go
        while let Some(event) = self.rx.try_next().ok().flatten() {
            changes.push(event);
        }

        // Filter the changes
        let mut all_mods: Vec<PathBuf> = vec![];

        // Decompose the events into a list of all the files that have changed
        for event in changes.drain(..) {
            // We only care about certain events.
            if !is_allowed_notify_event(&event) {
                continue;
            }

            for path in event.paths {
                all_mods.push(path.clone());
            }
        }

        // Collect the files that have changed
        let mut files = vec![];
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
                continue;
            }

            // If the path is ignored, don't watch it
            if self.ignore.matched(path, path.is_dir()).is_ignore() {
                tracing::info!("Ignoring update to file: {:?}", path);
                continue;
            }

            tracing::info!("Enqueuing hotreload update to file: {:?}", path);

            files.push(path.clone());
        }

        ServeUpdate::FilesChanged { files }
    }

    pub(crate) fn attempt_hot_reload(
        &mut self,
        modified_files: Vec<PathBuf>,
        runner: &AppRunner,
    ) -> Option<HotReloadMsg> {
        // If we have any changes to the rust files, we need to update the file map
        let crate_dir = self.krate.crate_dir();
        let mut templates = vec![];

        // Prepare the hotreload message we need to send
        let mut edited_rust_files = Vec::new();
        let mut assets = Vec::new();

        for path in modified_files {
            // for various assets that might be linked in, we just try to hotreloading them forcefully
            // That is, unless they appear in an include! macro, in which case we need to a full rebuild....
            let Some(ext) = path.extension().and_then(|v| v.to_str()) else {
                continue;
            };

            match ext {
                "rs" => edited_rust_files.push(path),

                // Look through the runners to see if any of them have an asset that matches the path
                _ => {
                    for runner in runner.running.values() {
                        if let Some(bundled_name) = runner.hotreload_asset(&path) {
                            assets.push(bundled_name);
                        }
                    }
                }
            }
        }

        assets.dedup();

        // Process the rust files
        for rust_file in edited_rust_files {
            match self.file_map.update_rsx::<HtmlCtx>(&rust_file, &crate_dir) {
                Ok(hotreloaded_templates) => {
                    templates.extend(hotreloaded_templates);
                }

                // If the file is not reloadable, we need to rebuild
                Err(HotreloadError::Notreloadable) => return None,

                // The rust file may have failed to parse, but that is most likely
                // because the user is in the middle of adding new code
                // We just ignore the error and let Rust analyzer warn about the problem
                Err(HotreloadError::Parse) => {}

                // Otherwise just log the error
                Err(err) => {
                    tracing::error!("Error hotreloading file {rust_file:?}: {err}")
                }
            }
        }

        let msg = HotReloadMsg { templates, assets };

        self.add_hot_reload_message(&msg);

        Some(msg)
    }

    /// Get any hot reload changes that have been applied since the last full rebuild
    pub(crate) fn applied_hot_reload_changes(&mut self) -> Option<HotReloadMsg> {
        self.applied_hot_reload_message.clone()
    }

    /// Clear the hot reload changes. This should be called any time a new build is starting
    pub(crate) fn clear_hot_reload_changes(&mut self) {
        self.applied_hot_reload_message.take();
    }

    /// Store the hot reload changes for any future clients that connect
    fn add_hot_reload_message(&mut self, msg: &HotReloadMsg) {
        let Some(applied) = &mut self.applied_hot_reload_message else {
            self.applied_hot_reload_message = Some(msg.clone());
            return;
        };

        // Merge the assets, unknown files, and templates
        // We keep the newer change if there is both a old and new change
        let mut templates: HashMap<String, _> = std::mem::take(&mut applied.templates)
            .into_iter()
            .map(|template| (template.location.clone(), template))
            .collect();
        let mut assets: HashSet<PathBuf> =
            std::mem::take(&mut applied.assets).into_iter().collect();
        for template in &msg.templates {
            templates.insert(template.location.clone(), template.clone());
        }

        assets.extend(msg.assets.iter().cloned());
        applied.templates = templates.into_values().collect();
        applied.assets = assets.into_iter().collect();
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

/// Tests if the provided [`notify::Event`] is something we listen to so we can avoid unescessary hot reloads.
fn is_allowed_notify_event(event: &notify::Event) -> bool {
    let allowed = match event.kind {
        EventKind::Modify(ModifyKind::Data(_)) => true,
        EventKind::Modify(ModifyKind::Name(_)) => true,
        EventKind::Create(_) => true,
        EventKind::Remove(_) => true,
        // The primary modification event on WSL's poll watcher.
        EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime)) => true,
        // Catch-all for unknown event types.
        EventKind::Modify(ModifyKind::Any) => true,
        // Don't care about anything else.
        _ => false,
    };

    tracing::info!("is_allowed_notify_event:  {allowed:?} for {event:#?}");

    allowed
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
