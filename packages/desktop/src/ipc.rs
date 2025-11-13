use serde::{Deserialize, Serialize};
use tao::window::WindowId;

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum UserWindowEvent {
    /// A global hotkey event
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    GlobalHotKeyEvent(global_hotkey::GlobalHotKeyEvent),

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    MudaMenuEvent(muda::MenuEvent),

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    TrayIconEvent(tray_icon::TrayIconEvent),

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    TrayMenuEvent(tray_icon::menu::MenuEvent),

    /// Poll the virtualdom
    Poll(WindowId),

    /// Handle an ipc message eminating from the window.postMessage of a given webview
    Ipc {
        id: WindowId,
        msg: IpcMessage,
    },

    /// Handle a hotreload event, basically telling us to update our templates
    #[cfg(all(feature = "devtools", debug_assertions))]
    HotReloadEvent(dioxus_devtools::DevserverMsg),

    // Windows-only drag-n-drop fix events.
    WindowsDragDrop(WindowId),
    WindowsDragOver(WindowId, i32, i32),
    WindowsDragLeave(WindowId),

    /// Create a new window
    NewWindow,

    /// Close a given window (could be any window!)
    CloseWindow(WindowId),

    /// Gracefully shutdown the entire app
    Shutdown,
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
    UserEvent,
    Query,
    BrowserOpen,
    Initialize,
    Other(&'a str),
}

impl IpcMessage {
    pub(crate) fn method(&self) -> IpcMethod<'_> {
        match self.method.as_str() {
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
