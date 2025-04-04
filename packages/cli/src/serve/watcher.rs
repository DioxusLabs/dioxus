use super::{update::ServeUpdate, AppRunner};
use crate::{cli::serve::ServeArgs, BuildRequest, Result};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use notify::{
    event::{MetadataKind, ModifyKind},
    Config, EventKind, RecursiveMode, Watcher as NotifyWatcher,
};
use std::{path::PathBuf, time::Duration};

/// This struct stores the file watcher and the filemap for the project.
///
/// This is where we do workspace discovery and recursively listen for changes in Rust files and asset
/// directories.
///
/// The watcher is not tightly integrated with the runner since each build likely brings in a similar
/// set of paths to watch and there is quite a large overhead to setting up the watcher.
///
/// Previously we manually walked the workspace and added all the paths to the watcher, but now
/// we use depinfo directly once the build is complete.
pub(crate) struct Watcher {
    rx: UnboundedReceiver<notify::Event>,
    _tx: UnboundedSender<notify::Event>,
    watcher: Box<dyn notify::Watcher>,
}

impl Watcher {
    pub(crate) async fn start(runner: &AppRunner) -> Result<Self> {
        todo!()
        // let (tx, rx) = futures_channel::mpsc::unbounded();

        // let mut watcher = Self {
        //     watcher: create_notify_watcher(serve, tx.clone()),
        //     _tx: tx,
        //     // krate: krate.clone(),
        //     rx,
        // };

        // watcher.watch_filesystem();

        // watcher
    }
}
