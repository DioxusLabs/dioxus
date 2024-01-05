use serde::{Deserialize, Serialize};
use tao::window::WindowId;

/// A pair of data
#[derive(Debug, Clone)]
pub struct UserWindowEvent(pub EventData, pub WindowId);

/// The data that might eminate from any window/webview
#[derive(Debug, Clone)]
pub enum EventData {
    /// Poll the virtualdom
    Poll,

    /// Handle an ipc message eminating from the window.postMessage of a given webview
    Ipc(IpcMessage),

    /// Handle a hotreload event, basically telling us to update our templates
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    HotReloadEvent(dioxus_hot_reload::HotReloadMsg),

    /// Create a new window
    NewWindow,

    /// Close a given window (could be any window!)
    CloseWindow,
}

/// A message struct that manages the communication between the webview and the eventloop code
///
/// This needs to be serializable across the JS boundary, so the method names and structs are sensitive.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct IpcMessage {
    method: String,
    params: serde_json::Value,
}

/// A set of known messages that we need to respond to
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum IpcMethod<'a> {
    FileDialog,
    UserEvent,
    Query,
    BrowserOpen,
    Initialize,
    Other(&'a str),
}

impl IpcMessage {
    pub(crate) fn method(&self) -> IpcMethod {
        match self.method.as_str() {
            // todo: this is a misspelling, needs to be fixed
            "file_diolog" => IpcMethod::FileDialog,
            "user_event" => IpcMethod::UserEvent,
            "query" => IpcMethod::Query,
            "browser_open" => IpcMethod::BrowserOpen,
            "initialize" => IpcMethod::Initialize,
            _ => IpcMethod::Other(&self.method),
        }
    }

    pub(crate) fn params(self) -> serde_json::Value {
        self.params
    }
}
