use crate::DesktopService;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::{mpsc::SyncSender, Arc, Mutex};
use tao::window::WindowId;
use wry_bindgen::runtime::WryBindgenEvent;

/// Type alias for the desktop service callback function.
pub(crate) type DesktopServiceCallback =
    Box<dyn FnOnce(&DesktopService) -> Box<dyn Any + Send> + Send>;

/// Inner type that holds the callback and response channel for DesktopService operations.
pub(crate) struct DesktopServiceCallbackInner {
    pub callback: DesktopServiceCallback,
    pub sender: SyncSender<Box<dyn Any + Send>>,
}

/// Wrapper for a callback that runs with DesktopService access on the main thread.
/// The inner Option allows taking the callback exactly once.
#[derive(Clone)]
pub struct DesktopServiceCallbackWrapper(
    pub(crate) Arc<Mutex<Option<DesktopServiceCallbackInner>>>,
);

impl std::fmt::Debug for DesktopServiceCallbackWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DesktopServiceCallbackWrapper")
            .field(&"...")
            .finish()
    }
}

impl DesktopServiceCallbackWrapper {
    pub(crate) fn new(
        callback: DesktopServiceCallback,
        sender: SyncSender<Box<dyn Any + Send>>,
    ) -> Self {
        Self(Arc::new(Mutex::new(Some(DesktopServiceCallbackInner {
            callback,
            sender,
        }))))
    }

    pub(crate) fn take(&self) -> Option<DesktopServiceCallbackInner> {
        self.0.lock().ok()?.take()
    }
}

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

    /// wry-bindgen IPC event (wrapped for Clone compatibility)
    WryBindgenEvent(WryBindgenEvent),

    /// Run a closure with access to a specific window's DesktopService on the main thread
    RunWithDesktopService {
        /// The window ID to get the DesktopService for
        id: WindowId,
        /// The callback wrapper containing the closure and response channel
        callback: DesktopServiceCallbackWrapper,
    },
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
    BrowserOpen,
    Initialize,
    Other(&'a str),
}

impl IpcMessage {
    pub(crate) fn method(&self) -> IpcMethod<'_> {
        match self.method.as_str() {
            "user_event" => IpcMethod::UserEvent,
            "browser_open" => IpcMethod::BrowserOpen,
            "initialize" => IpcMethod::Initialize,
            _ => IpcMethod::Other(&self.method),
        }
    }

    pub(crate) fn params(self) -> serde_json::Value {
        self.params
    }
}
