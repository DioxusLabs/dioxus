use std::cell::RefCell;
use std::rc::Rc;

use crate::eval::EvalResult;
use crate::events::IpcMessage;
use crate::Config;
use dioxus_core::ScopeState;
use dioxus_core::VirtualDom;
use serde_json::Value;
use wry::application::event_loop::EventLoopProxy;
#[cfg(target_os = "ios")]
use wry::application::platform::ios::WindowExtIOS;
use wry::application::window::Fullscreen as WryFullscreen;
use wry::application::window::Window;
use wry::application::window::WindowId;
use wry::webview::WebView;

pub type ProxyType = EventLoopProxy<UserWindowEvent>;

/// Get an imperative handle to the current window
pub fn use_window(cx: &ScopeState) -> &DesktopContext {
    cx.use_hook(|| cx.consume_context::<DesktopContext>())
        .as_ref()
        .unwrap()
}

pub type WebviewQueue = Rc<RefCell<Vec<(VirtualDom, crate::cfg::Config)>>>;

/// An imperative interface to the current window.
///
/// To get a handle to the current window, use the [`use_window`] hook.
///
///
/// # Example
///
/// you can use `cx.consume_context::<DesktopContext>` to get this context
///
/// ```rust, ignore
///     let desktop = cx.consume_context::<DesktopContext>().unwrap();
/// ```
#[derive(Clone)]
pub struct DesktopContext {
    /// The wry/tao proxy to the current window
    pub webview: Rc<WebView>,

    /// The proxy to the event loop
    pub proxy: ProxyType,

    /// The receiver for eval results since eval is async
    pub(super) eval: tokio::sync::broadcast::Sender<Value>,

    pub(super) pending_windows: WebviewQueue,

    #[cfg(target_os = "ios")]
    pub(crate) views: Rc<RefCell<Vec<*mut objc::runtime::Object>>>,
}

/// A smart pointer to the current window.
impl std::ops::Deref for DesktopContext {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        self.webview.window()
    }
}

impl DesktopContext {
    pub(crate) fn new(webview: Rc<WebView>, proxy: ProxyType, webviews: WebviewQueue) -> Self {
        Self {
            webview,
            proxy,
            eval: tokio::sync::broadcast::channel(8).0,
            pending_windows: webviews,
            #[cfg(target_os = "ios")]
            views: Default::default(),
        }
    }

    /// Create a new window using the props and window builder
    pub fn new_window<T: 'static>(&self, dom: VirtualDom, cfg: Config) {
        self.pending_windows.borrow_mut().push((dom, cfg));
        self.proxy
            .send_event(UserWindowEvent(EventData::NewWindow, self.id()))
            .unwrap();
    }

    /// trigger the drag-window event
    ///
    /// Moves the window with the left mouse button until the button is released.
    ///
    /// you need use it in `onmousedown` event:
    /// ```rust, ignore
    /// onmousedown: move |_| { desktop.drag_window(); }
    /// ```
    pub fn drag(&self) {
        let window = self.webview.window();

        // if the drag_window has any errors, we don't do anything
        window.fullscreen().is_none().then(|| window.drag_window());
    }

    /// Toggle whether the window is maximized or not
    pub fn toggle_maximized(&self) {
        let window = self.webview.window();

        window.set_maximized(!window.is_maximized())
    }

    /// close window
    pub fn close(&self) {
        let _ = self
            .proxy
            .send_event(UserWindowEvent(EventData::CloseWindow, self.id()));
    }

    /// change window to fullscreen
    pub fn set_fullscreen(&self, fullscreen: bool) {
        if let Some(handle) = self.webview.window().current_monitor() {
            self.webview
                .window()
                .set_fullscreen(fullscreen.then_some(WryFullscreen::Borderless(Some(handle))));
        }
    }

    /// launch print modal
    pub fn print(&self) {
        if let Err(e) = self.webview.print() {
            log::warn!("Open print modal failed: {e}");
        }
    }

    /// Set the zoom level of the webview
    pub fn set_zoom_level(&self, level: f64) {
        self.webview.zoom(level);
    }

    /// opens DevTool window
    pub fn devtool(&self) {
        #[cfg(debug_assertions)]
        self.webview.open_devtools();

        #[cfg(not(debug_assertions))]
        log::warn!("Devtools are disabled in release builds");
    }

    /// Evaluate a javascript expression
    pub fn eval(&self, code: &str) -> EvalResult {
        // Embed the return of the eval in a function so we can send it back to the main thread
        let script = format!(
            r#"
            window.ipc.postMessage(
                JSON.stringify({{
                    "method":"eval_result",
                    "params": (
                        function(){{
                            {}
                        }}
                    )()
                }})
            );
            "#,
            code
        );

        if let Err(e) = self.webview.evaluate_script(&script) {
            // send an error to the eval receiver
            log::warn!("Eval script error: {e}");
        }

        EvalResult::new(self.eval.clone())
    }

    /// Push an objc view to the window
    #[cfg(target_os = "ios")]
    pub fn push_view(&self, view: objc_id::ShareId<objc::runtime::Object>) {
        let window = self.webview.window();

        unsafe {
            use objc::runtime::Object;
            use objc::*;
            assert!(is_main_thread());
            let ui_view = window.ui_view() as *mut Object;
            let ui_view_frame: *mut Object = msg_send![ui_view, frame];
            let _: () = msg_send![view, setFrame: ui_view_frame];
            let _: () = msg_send![view, setAutoresizingMask: 31];

            let ui_view_controller = window.ui_view_controller() as *mut Object;
            let _: () = msg_send![ui_view_controller, setView: view];
            self.views.borrow_mut().push(ui_view);
        }
    }

    /// Pop an objc view from the window
    #[cfg(target_os = "ios")]
    pub fn pop_view(&self) {
        let window = self.webview.window();

        unsafe {
            use objc::runtime::Object;
            use objc::*;
            assert!(is_main_thread());
            if let Some(view) = self.views.borrow_mut().pop() {
                let ui_view_controller = window.ui_view_controller() as *mut Object;
                let _: () = msg_send![ui_view_controller, setView: view];
            }
        }
    }
}

#[derive(Debug)]
pub struct UserWindowEvent(pub EventData, pub WindowId);

#[derive(Debug)]
pub enum EventData {
    Poll,

    Ipc(IpcMessage),

    NewWindow,

    CloseWindow,
}

#[cfg(target_os = "ios")]
fn is_main_thread() -> bool {
    use objc::runtime::{Class, BOOL, NO};
    use objc::*;

    let cls = Class::get("NSThread").unwrap();
    let result: BOOL = unsafe { msg_send![cls, isMainThread] };
    result != NO
}
