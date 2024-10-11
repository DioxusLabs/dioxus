use super::detect::is_wsl;
use super::update::ServeUpdate;
use crate::{cli::serve::ServeArgs, dioxus_crate::DioxusCrate};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use ignore::gitignore::Gitignore;
use notify::{
    event::{MetadataKind, ModifyKind},
    Config, EventKind,
};
use std::{path::PathBuf, time::Duration};

/// This struct stores the file watcher and the filemap for the project.
///
/// This is where we do workspace discovery and recursively listen for changes in Rust files and asset
/// directories.
pub(crate) struct Watcher {
    ignore: Gitignore,
    rx: UnboundedReceiver<notify::Event>,
    _krate: DioxusCrate,
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
        let mut allow_watch_path = krate.config.web.watcher.watch_path.clone();
        allow_watch_path.push(krate.config.application.asset_dir.clone());
        allow_watch_path.push("Cargo.toml".to_string().into());
        allow_watch_path.push("Dioxus.toml".to_string().into());
        allow_watch_path.push("assets".to_string().into());
        allow_watch_path.dedup();

        // Build the event handler for notify.
        let notify_event_handler = {
            let tx = tx.clone();
            move |info: notify::Result<notify::Event>| {
                if let Ok(e) = info {
                    let is_allowed_notify_event = is_allowed_notify_event(&e);
                    if is_allowed_notify_event {
                        _ = tx.unbounded_send(e);
                    }
                }
            }
        };

        const NOTIFY_ERROR_MSG: &str = "Failed to create file watcher.\nEnsure you have the required permissions to watch the specified directories.";

        // Create the file watcher.
        // If we are in WSL, we must use Notify's poll watcher due to an event propagation issue.
        let mut watcher: Box<dyn notify::Watcher> = match is_wsl() {
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

        let ignore = krate.gitignore();

        // Watch the specified paths
        // todo: make sure we don't double-watch paths if they're nested
        for sub_path in allow_watch_path {
            let path = &krate.crate_dir().join(sub_path);

            // If the path is ignored, don't watch it
            if ignore.matched(path, path.is_dir()).is_ignore() {
                continue;
            }

            tracing::debug!("Watching path {path:?}");

            let mode = notify::RecursiveMode::Recursive;

            if let Err(err) = watcher.watch(path, mode) {
                tracing::debug!("Failed to watch path: {}", err);
            }
        }

        Self {
            _tx: tx,
            _krate: krate.clone(),
            rx,
            _watcher: watcher,
            ignore,
            _last_update_time: chrono::Local::now().timestamp(),
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
                continue;
            }

            files.push(path.clone());
        }

        tracing::debug!("Files changed: {files:?}");

        ServeUpdate::FilesChanged { files }
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
        // The primary modification event on WSL's poll watcher.
        EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime)) => true,
        // Catch-all for unknown event types.
        EventKind::Modify(ModifyKind::Any) => false,
        EventKind::Modify(ModifyKind::Metadata(_)) => false,
        // EventKind::Modify(ModifyKind::Any) => true,
        // Don't care about anything else.
        EventKind::Create(_) => true,
        EventKind::Remove(_) => true,
        _ => false,
    };

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
