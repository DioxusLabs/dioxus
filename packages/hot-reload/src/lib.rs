use dioxus_rsx::HotReloadedTemplate;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "apply")]
mod apply;

#[cfg(feature = "apply")]
pub use apply::*;

#[cfg(feature = "serve")]
mod ws_receiver;

#[cfg(feature = "serve")]
pub use ws_receiver::*;

/// The script to inject into the page to reconnect to server if the connection is lost
#[cfg(feature = "serve")]
pub const RECONNECT_SCRIPT: &str = include_str!("assets/autoreload.js");

/// The endpoint the dev server listens on
/// Not configurable
pub const HOTRELOAD_ENDPOINT: &str = "/_dioxus";

/// A message the hot reloading server sends to the client
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(bound(deserialize = "'de: 'static"))]
pub enum DevserverMsg {
    /// Attempt a hotreload
    /// This includes all the templates/literals/assets/binary patches that have changed in one shot
    HotReload(HotReloadMsg),

    /// The program is shutting down completely - maybe toss up a splash screen or something?
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct HotReloadMsg {
    pub templates: Vec<HotReloadedTemplate>,
    pub assets: Vec<PathBuf>,
}
