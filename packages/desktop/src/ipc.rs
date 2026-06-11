use crate::DesktopService;
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use tao::window::WindowId;
use tokio::sync::oneshot;

type DesktopServiceCallbackFn = Box<dyn FnOnce(&DesktopService) + Send>;

pub struct DesktopServiceCallback {
    callback: DesktopServiceCallbackFn,
}

impl std::fmt::Debug for DesktopServiceCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad("DesktopServiceCallback { .. }")
    }
}

impl DesktopServiceCallback {
    pub(crate) fn new<T, F>(f: F) -> (Self, oneshot::Receiver<T>)
    where
        T: Send + 'static,
        F: FnOnce(&DesktopService) -> T + Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        let callback: DesktopServiceCallbackFn = Box::new(move |desktop| {
            let _ = sender.send(f(desktop));
        });

        (Self { callback }, receiver)
    }

    pub(crate) fn new_blocking<T, F>(f: F) -> (Self, mpsc::Receiver<T>)
    where
        T: Send + 'static,
        F: FnOnce(&DesktopService) -> T + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel();
        let callback: DesktopServiceCallbackFn = Box::new(move |desktop| {
            let _ = sender.send(f(desktop));
        });

        (Self { callback }, receiver)
    }

    pub(crate) fn run(self, desktop: &DesktopService) {
        (self.callback)(desktop);
    }
}

#[derive(Debug)]
pub struct UserWindowEvent {
    variant: UserWindowEventVariant,
}

impl UserWindowEvent {
    pub(crate) fn into_variant(self) -> UserWindowEventVariant {
        self.variant
    }

    pub(crate) fn variant(&self) -> &UserWindowEventVariant {
        &self.variant
    }

    pub(crate) fn run_with_desktop_service(
        window_id: WindowId,
        callback: DesktopServiceCallback,
    ) -> Self {
        UserWindowEventVariant::RunWithDesktopService {
            window_id,
            callback,
        }
        .into()
    }
}

/// `UserWindowEvent` is just a public wrapper hiding the `pub(crate)` variants; crate code builds
/// one straight from a variant.
impl From<UserWindowEventVariant> for UserWindowEvent {
    fn from(variant: UserWindowEventVariant) -> Self {
        Self { variant }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub(crate) enum UserWindowEventVariant {
    /// A global hotkey event
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    GlobalHotKeyEvent(global_hotkey::GlobalHotKeyEvent),

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    MudaMenuEvent(muda::MenuEvent),

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    TrayIconEvent(tray_icon::TrayIconEvent),

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    TrayMenuEvent(tray_icon::menu::MenuEvent),

    /// Re-point every webview's edit websocket after the OS killed the socket
    /// (e.g. iOS sleep) and the server rebound to a new port.
    ReconnectEdits,

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

    /// Destroy a window unconditionally, ignoring [`WindowCloseBehaviour`].
    ///
    /// Sent when a window's VirtualDom task has finished: hiding such a window
    /// (`WindowCloseBehaviour::WindowHides`) would leave an unrecoverable zombie.
    ///
    /// [`WindowCloseBehaviour`]: crate::WindowCloseBehaviour
    DestroyWindow(WindowId),

    /// Gracefully shutdown the entire app
    Shutdown,

    /// Wake the wry-bindgen driver for a webview.
    WryBindgenDriverWake(WindowId),

    /// Run a closure with access to a specific window's DesktopService on the main thread
    RunWithDesktopService {
        /// The window ID to get the DesktopService for
        window_id: WindowId,

        /// The callback containing the closure and response channel
        callback: DesktopServiceCallback,
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
    BrowserOpen,
    Initialize,
    Other(&'a str),
}

impl IpcMessage {
    pub(crate) fn method(&self) -> IpcMethod<'_> {
        match self.method.as_str() {
            "browser_open" => IpcMethod::BrowserOpen,
            "initialize" => IpcMethod::Initialize,
            _ => IpcMethod::Other(&self.method),
        }
    }

    pub(crate) fn params(self) -> serde_json::Value {
        self.params
    }
}
