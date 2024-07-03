use dioxus_rsx::HotReloadedTemplate;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "serve")]
mod ws_receiver;

#[cfg(feature = "serve")]
pub use ws_receiver::*;

/// The script to inject into the page to reconnect to server if the connection is lost
#[cfg(feature = "serve")]
pub const RECONNECT_SCRIPT: &str = include_str!("assets/autoreload.js");

#[cfg(feature = "file_watcher")]
pub use dioxus_html::HtmlCtx;

/// A message the hot reloading server sends to the client
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(bound(deserialize = "'de: 'static"))]
pub enum DevserverMsg {
    /// Attempt a hotreload
    /// This includes all the templates/literals/assets/binary patches that have changed in one shot
    HotReload(HotReloadMsg),

    /// The program needs to reloaded somehow - you should preserve your state and reload
    Reload,

    /// The program is shutting down completely - maybe toss up a splash screen or something?
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct HotReloadMsg {
    pub templates: Vec<HotReloadedTemplate>,
    pub assets: Vec<PathBuf>,
}
