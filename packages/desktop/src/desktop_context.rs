use std::rc::Rc;

use dioxus_core::ScopeState;
use wry::application::event_loop::EventLoopProxy;

use crate::user_window_events::UserWindowEvent;
use UserWindowEvent::*;

type ProxyType = EventLoopProxy<UserWindowEvent>;

/// Desktop-Window handle api context
///
/// you can use this context control some window event
///
/// you can use `cx.consume_context::<DesktopContext>` to get this context
///
/// ```rust
///     let desktop = cx.consume_context::<DesktopContext>().unwrap();
/// ```
#[derive(Clone)]
pub struct DesktopContext {
    proxy: ProxyType,
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

    // set cursor visible or not
    pub fn set_cursor_visible(&self, visible: bool) {
        let _ = self.proxy.send_event(CursorVisible(visible));
    }

    // set cursor grab
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

/// use this function can get the `DesktopContext` context.
pub fn use_window(cx: &ScopeState) -> &Rc<DesktopContext> {
    cx.use_hook(|_| cx.consume_context::<DesktopContext>())
        .as_ref()
        .unwrap()
}
