use dioxus_core::internal::HotReloadTemplateWithLocation;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A message the hot reloading server sends to the client
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum DevserverMsg {
    /// Attempt a hotreload
    /// This includes all the templates/literals/assets/binary patches that have changed in one shot
    HotReload(HotReloadMsg),

    /// The devserver is starting a full rebuild.
    FullReloadStart,

    /// The full reload failed.
    FullReloadFailed,

    /// The app should reload completely if it can
    FullReloadCommand,

    /// The program is shutting down completely - maybe toss up a splash screen or something?
    Shutdown,
}

/// A message the client sends from the frontend to the devserver
///
/// This is used to communicate with the devserver
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ClientMsg {
    Log {
        level: String,
        messages: Vec<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct HotReloadMsg {
    pub templates: Vec<HotReloadTemplateWithLocation>,
    pub assets: Vec<PathBuf>,

    /// A file changed that's not an asset or a rust file - best of luck!
    pub unknown_files: Vec<PathBuf>,
}
