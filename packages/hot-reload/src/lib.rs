use dioxus_rsx::HotReloadedTemplate;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "client")]
mod client;

#[cfg(feature = "client")]
pub use client::*;

#[cfg(feature = "serve")]
mod ws_receiver;

#[cfg(feature = "serve")]
pub use ws_receiver::*;

/// A message the hot reloading server sends to the client
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(bound(deserialize = "'de: 'static"))]
pub enum DevserverMsg {
    /// Attempt a hotreload
    /// This includes all the templates/literals/assets/binary patches that have changed in one shot
    HotReload(HotReloadMsg),

    /// The app should reload completely if it can
    FullReload,

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
#[serde(bound(deserialize = "'de: 'static"))]
pub struct HotReloadMsg {
    pub templates: Vec<HotReloadedTemplate>,
    pub assets: Vec<PathBuf>,

    /// A file changed that's not an asset or a rust file - best of luck!
    pub unknown_files: Vec<PathBuf>,
}

#[test]
fn serialize_client_msg() {
    let msg = ClientMsg::Log {
        level: "info".to_string(),
        messages: vec!["hello world".to_string()],
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert_eq!(
        json,
        r#"{"Log":{"level":"info","messages":["hello world"]}}"#
    );
}
