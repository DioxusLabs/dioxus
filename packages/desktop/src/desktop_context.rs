use std::rc::Rc;

use crate::controller::DesktopController;
use dioxus_core::ScopeState;
use serde::de::Error;
use serde_json::Value;
use std::future::Future;
use std::future::IntoFuture;
use std::pin::Pin;
use wry::application::event_loop::ControlFlow;
use wry::application::event_loop::EventLoopProxy;
#[cfg(target_os = "ios")]
use wry::application::platform::ios::WindowExtIOS;
use wry::application::window::Fullscreen as WryFullscreen;

use UserWindowEvent::*;

pub type ProxyType = EventLoopProxy<UserWindowEvent>;

/// Get an imperative handle to the current window
pub fn use_window(cx: &ScopeState) -> &DesktopContext {
    cx.use_hook(|| cx.consume_context::<DesktopContext>())
        .as_ref()
        .unwrap()
}

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
    pub proxy: ProxyType,
    pub(super) eval_reciever: Rc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<Value>>>,
}

impl DesktopContext {
    pub(crate) fn new(
        proxy: ProxyType,
        eval_reciever: tokio::sync::mpsc::UnboundedReceiver<Value>,
    ) -> Self {
        Self {
            proxy,
            eval_reciever: Rc::new(tokio::sync::Mutex::new(eval_reciever)),
        }
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
        let _ = self.proxy.send_event(DragWindow);
    }

    /// set window minimize state
    pub fn set_minimized(&self, minimized: bool) {
        let _ = self.proxy.send_event(Minimize(minimized));
    }

    /// set window maximize state
    pub fn set_maximized(&self, maximized: bool) {
        let _ = self.proxy.send_event(Maximize(maximized));
    }

    /// toggle window maximize state
    pub fn toggle_maximized(&self) {
        let _ = self.proxy.send_event(MaximizeToggle);
    }

    /// set window visible or not
    pub fn set_visible(&self, visible: bool) {
        let _ = self.proxy.send_event(Visible(visible));
    }

    /// close window
    pub fn close(&self) {
        let _ = self.proxy.send_event(CloseWindow);
    }

    /// set window to focus
    pub fn focus(&self) {
        let _ = self.proxy.send_event(FocusWindow);
    }

    /// change window to fullscreen
    pub fn set_fullscreen(&self, fullscreen: bool) {
        let _ = self.proxy.send_event(Fullscreen(fullscreen));
    }

    /// set resizable state
    pub fn set_resizable(&self, resizable: bool) {
        let _ = self.proxy.send_event(Resizable(resizable));
    }

    /// set the window always on top
    pub fn set_always_on_top(&self, top: bool) {
        let _ = self.proxy.send_event(AlwaysOnTop(top));
    }

    /// set cursor visible or not
    pub fn set_cursor_visible(&self, visible: bool) {
        let _ = self.proxy.send_event(CursorVisible(visible));
    }

    /// set cursor grab
    pub fn set_cursor_grab(&self, grab: bool) {
        let _ = self.proxy.send_event(CursorGrab(grab));
    }

    /// set window title
    pub fn set_title(&self, title: &str) {
        let _ = self.proxy.send_event(SetTitle(String::from(title)));
    }

    /// change window to borderless
    pub fn set_decorations(&self, decoration: bool) {
        let _ = self.proxy.send_event(SetDecorations(decoration));
    }

    /// set window zoom level
    pub fn set_zoom_level(&self, scale_factor: f64) {
        let _ = self.proxy.send_event(SetZoomLevel(scale_factor));
    }

    /// launch print modal
    pub fn print(&self) {
        let _ = self.proxy.send_event(Print);
    }

    /// opens DevTool window
    pub fn devtool(&self) {
        let _ = self.proxy.send_event(DevTool);
    }

    /// run (evaluate) a script in the WebView context
    pub fn eval(&self, script: impl std::string::ToString) {
        let _ = self.proxy.send_event(Eval(script.to_string()));
    }

    /// Push view
    #[cfg(target_os = "ios")]
    pub fn push_view(&self, view: objc_id::ShareId<objc::runtime::Object>) {
        let _ = self.proxy.send_event(PushView(view));
    }

    /// Push view
    #[cfg(target_os = "ios")]
    pub fn pop_view(&self) {
        let _ = self.proxy.send_event(PopView);
    }
}

#[derive(Debug)]
pub enum UserWindowEvent {
    EditsReady,
    Initialize,

    CloseWindow,
    DragWindow,
    FocusWindow,

    Visible(bool),
    Minimize(bool),
    Maximize(bool),
    MaximizeToggle,
    Resizable(bool),
    AlwaysOnTop(bool),
    Fullscreen(bool),

    CursorVisible(bool),
    CursorGrab(bool),

    SetTitle(String),
    SetDecorations(bool),

    SetZoomLevel(f64),

    Print,
    DevTool,

    Eval(String),

    #[cfg(target_os = "ios")]
    PushView(objc_id::ShareId<objc::runtime::Object>),
    #[cfg(target_os = "ios")]
    PopView,
}

impl DesktopController {
    pub(super) fn handle_event(
        &mut self,
        user_event: UserWindowEvent,
        control_flow: &mut ControlFlow,
    ) {
        // currently dioxus-desktop supports a single window only,
        // so we can grab the only webview from the map;
        // on wayland it is possible that a user event is emitted
        // before the webview is initialized. ignore the event.
        let webview = if let Some(webview) = self.webviews.values().next() {
            webview
        } else {
            return;
        };

        let window = webview.window();

        match user_event {
            Initialize | EditsReady => self.try_load_ready_webviews(),
            CloseWindow => *control_flow = ControlFlow::Exit,
            DragWindow => {
                // if the drag_window has any errors, we don't do anything
                window.fullscreen().is_none().then(|| window.drag_window());
            }
            Visible(state) => window.set_visible(state),
            Minimize(state) => window.set_minimized(state),
            Maximize(state) => window.set_maximized(state),
            MaximizeToggle => window.set_maximized(!window.is_maximized()),
            Fullscreen(state) => {
                if let Some(handle) = window.current_monitor() {
                    window.set_fullscreen(state.then_some(WryFullscreen::Borderless(Some(handle))));
                }
            }
            FocusWindow => window.set_focus(),
            Resizable(state) => window.set_resizable(state),
            AlwaysOnTop(state) => window.set_always_on_top(state),

            Eval(code) => {
                let script = format!(
                    r#"window.ipc.postMessage(JSON.stringify({{"method":"eval_result", params: (function(){{
                        {}
                    }})()}}));"#,
                    code
                );
                if let Err(e) = webview.evaluate_script(&script) {
                    // we can't panic this error.
                    log::warn!("Eval script error: {e}");
                }
            }
            CursorVisible(state) => window.set_cursor_visible(state),
            CursorGrab(state) => {
                let _ = window.set_cursor_grab(state);
            }

            SetTitle(content) => window.set_title(&content),
            SetDecorations(state) => window.set_decorations(state),

            SetZoomLevel(scale_factor) => webview.zoom(scale_factor),

            Print => {
                if let Err(e) = webview.print() {
                    // we can't panic this error.
                    log::warn!("Open print modal failed: {e}");
                }
            }
            DevTool => {
                #[cfg(debug_assertions)]
                webview.open_devtools();
                #[cfg(not(debug_assertions))]
                log::warn!("Devtools are disabled in release builds");
            }

            #[cfg(target_os = "ios")]
            PushView(view) => unsafe {
                use objc::runtime::Object;
                use objc::*;
                assert!(is_main_thread());
                let ui_view = window.ui_view() as *mut Object;
                let ui_view_frame: *mut Object = msg_send![ui_view, frame];
                let _: () = msg_send![view, setFrame: ui_view_frame];
                let _: () = msg_send![view, setAutoresizingMask: 31];

                let ui_view_controller = window.ui_view_controller() as *mut Object;
                let _: () = msg_send![ui_view_controller, setView: view];
                self.views.push(ui_view);
            },

            #[cfg(target_os = "ios")]
            PopView => unsafe {
                use objc::runtime::Object;
                use objc::*;
                assert!(is_main_thread());
                if let Some(view) = self.views.pop() {
                    let ui_view_controller = window.ui_view_controller() as *mut Object;
                    let _: () = msg_send![ui_view_controller, setView: view];
                }
            },
        }
    }
}

/// Get a closure that executes any JavaScript in the WebView context.
pub fn use_eval<S: std::string::ToString>(cx: &ScopeState) -> &dyn Fn(S) -> EvalResult {
    let desktop = use_window(cx).clone();
    cx.use_hook(|| {
        move |script| {
            desktop.eval(script);
            let recv = desktop.eval_reciever.clone();
            EvalResult { reciever: recv }
        }
    })
}

/// A future that resolves to the result of a JavaScript evaluation.
pub struct EvalResult {
    reciever: Rc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>>>,
}

impl IntoFuture for EvalResult {
    type Output = Result<serde_json::Value, serde_json::Error>;

    type IntoFuture = Pin<Box<dyn Future<Output = Result<serde_json::Value, serde_json::Error>>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let mut reciever = self.reciever.lock().await;
            match reciever.recv().await {
                Some(result) => Ok(result),
                None => Err(serde_json::Error::custom("No result returned")),
            }
        }) as Pin<Box<dyn Future<Output = Result<serde_json::Value, serde_json::Error>>>>
    }
}

#[cfg(target_os = "ios")]
fn is_main_thread() -> bool {
    use objc::runtime::{Class, BOOL, NO};
    use objc::*;

    let cls = Class::get("NSThread").unwrap();
    let result: BOOL = unsafe { msg_send![cls, isMainThread] };
    result != NO
}
