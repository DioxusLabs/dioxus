use dioxus_core::internal::HotReloadTemplateWithLocation;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use subsecond_types::JumpTable;

/// A message the hot reloading server sends to the client
#[non_exhaustive]
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
#[non_exhaustive]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ClientMsg {
    Initialize {
        build_id: u64,
        aslr_reference: u64,
    },
    Log {
        level: String,
        messages: Vec<String>,
    },
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct HotReloadMsg {
    pub jump_table: Option<JumpTable>,
    pub templates: Vec<HotReloadTemplateWithLocation>,
    pub assets: Vec<PathBuf>,
}

impl HotReloadMsg {
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty() && self.assets.is_empty()
    }
}
