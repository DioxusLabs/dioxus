use super::detect::is_wsl;
use super::update::ServeUpdate;
use crate::dioxus_crate::DioxusCrate;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use ignore::gitignore::Gitignore;
use notify::{
    event::{MetadataKind, ModifyKind},
    Config, EventKind, RecursiveMode, Watcher as NotifyWatcher,
};
use std::{path::PathBuf, time::Duration};

/// This struct stores the file watcher and the filemap for the project.
///
/// This is where we do workspace discovery and recursively listen for changes in Rust files and asset
/// directories.
pub(crate) struct Watcher {
    rx: UnboundedReceiver<notify::Event>,
    ignore: Gitignore,
    krate: DioxusCrate,
    _tx: UnboundedSender<notify::Event>,
    watcher: Box<dyn notify::Watcher>,
}

impl Watcher {
    pub(crate) fn start(krate: &DioxusCrate) -> Self {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        let mut watcher = Self {
            watcher: create_notify_watcher(krate, tx.clone()),
            _tx: tx,
            krate: krate.clone(),
            rx,
            ignore: krate.workspace_gitignore(),
        };

        watcher.watch_filesystem();

        watcher
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
            // Make sure we add new folders to the watch list, provided they're not matched by the ignore list
            // We'll only watch new folders that are found under the crate, and then update our watcher to watch them
            // This unfortunately won't pick up new krates added "at a distance" - IE krates not within the workspace.
            if let EventKind::Create(_create_kind) = event.kind {
                // If it's a new folder, watch it
                // If it's a new cargo.toml (ie dep on the fly),
                // todo(jon) support new folders on the fly
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

            // If the path is ignored, don't watch it
            if self.ignore.matched(path, path.is_dir()).is_ignore() {
                continue;
            }

            files.push(path.clone());
        }

        tracing::debug!("Files changed: {files:?}");

        ServeUpdate::FilesChanged { files }
    }

    fn watch_filesystem(&mut self) {
        // Watch the folders of the crates that we're interested in
        for path in self.krate.watch_paths() {
            tracing::debug!("Watching path {path:?}");

            if let Err(err) = self.watcher.watch(&path, RecursiveMode::Recursive) {
                handle_notify_error(err);
            }
        }

        // Also watch the crates themselves, but not recursively, such that we can pick up new folders
        for krate in self.krate.all_watched_crates() {
            tracing::debug!("Watching path {krate:?}");
            if let Err(err) = self.watcher.watch(&krate, RecursiveMode::NonRecursive) {
                handle_notify_error(err);
            }
        }

        // Also watch the workspace dir, non recursively, such that we can pick up new folders there too
        if let Err(err) = self
            .watcher
            .watch(&self.krate.workspace_dir(), RecursiveMode::NonRecursive)
        {
            handle_notify_error(err);
        }
    }
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

fn create_notify_watcher(
    krate: &DioxusCrate,
    tx: UnboundedSender<notify::Event>,
) -> Box<dyn NotifyWatcher> {
    // Build the event handler for notify.
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

    if !is_wsl() {
        return Box::new(notify::recommended_watcher(handler).expect(NOTIFY_ERROR_MSG));
    }

    let poll_interval = Duration::from_secs(krate.settings.get_wsl_file_poll_interval() as u64);

    Box::new(
        notify::PollWatcher::new(handler, Config::default().with_poll_interval(poll_interval))
            .expect(NOTIFY_ERROR_MSG),
    )
}
