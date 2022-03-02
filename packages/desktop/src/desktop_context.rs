use crate::controller::DesktopController;
use dioxus_core::ScopeState;
use wry::application::event_loop::ControlFlow;
use wry::application::event_loop::EventLoopProxy;
use wry::application::window::Fullscreen as WryFullscreen;

use UserWindowEvent::*;

pub type ProxyType = EventLoopProxy<UserWindowEvent>;

/// Get an imperative handle to the current window
pub fn use_window(cx: &ScopeState) -> &DesktopContext {
    cx.use_hook(|_| cx.consume_context::<DesktopContext>())
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
/// ```rust
///     let desktop = cx.consume_context::<DesktopContext>().unwrap();
/// ```
#[derive(Clone)]
pub struct DesktopContext {
    /// The wry/tao proxy to the current window
    pub proxy: ProxyType,
}

impl DesktopContext {
    pub(crate) fn new(proxy: ProxyType) -> Self {
        Self { proxy }
    }

    /// trigger the drag-window event
    ///
    /// Moves the window with the left mouse button until the button is released.
    ///
    /// you need use it in `onmousedown` event:
    /// ```rust
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

    /// opens DevTool window
    pub fn devtool(&self) {
        let _ = self.proxy.send_event(DevTool);
    }
}

#[derive(Debug)]
pub enum UserWindowEvent {
    Update,

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

    DevTool,
}

pub(super) fn handler(
    user_event: UserWindowEvent,
    desktop: &mut DesktopController,
    control_flow: &mut ControlFlow,
) {
    // currently dioxus-desktop supports a single window only,
    // so we can grab the only webview from the map;
    let webview = desktop.webviews.values().next().unwrap();
    let window = webview.window();

    match user_event {
        Update => desktop.try_load_ready_webviews(),
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
                window.set_fullscreen(state.then(|| WryFullscreen::Borderless(Some(handle))));
            }
        }
        FocusWindow => window.set_focus(),
        Resizable(state) => window.set_resizable(state),
        AlwaysOnTop(state) => window.set_always_on_top(state),

        CursorVisible(state) => window.set_cursor_visible(state),
        CursorGrab(state) => {
            let _ = window.set_cursor_grab(state);
        }

        SetTitle(content) => window.set_title(&content),
        SetDecorations(state) => window.set_decorations(state),

        DevTool => webview.devtool(),
    }
}
