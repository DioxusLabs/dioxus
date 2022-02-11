use std::rc::Rc;

use dioxus_core::ScopeState;
use wry::application::{event_loop::EventLoopProxy, window::Fullscreen};

use crate::UserWindowEvent;

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
    pub fn drag_window(&self) {
        let _ = self.proxy.send_event(UserWindowEvent::DragWindow);
    }

    /// set window minimize state
    pub fn set_minimized(&self, minimized: bool) {
        let _ = self.proxy.send_event(UserWindowEvent::Minimize(minimized));
    }

    /// set window maximize state
    pub fn set_maximized(&self, maximized: bool) {
        let _ = self.proxy.send_event(UserWindowEvent::Maximize(maximized));
    }

    /// set window visible or not
    pub fn set_visible(&self, visible: bool) {
        let _ = self.proxy.send_event(UserWindowEvent::Visible(visible));
    }

    /// close window
    pub fn close_window(&self) {
        let _ = self.proxy.send_event(UserWindowEvent::CloseWindow);
    }

    /// set window to focus
    pub fn set_focus(&self) {
        let _ = self.proxy.send_event(UserWindowEvent::FocusWindow);
    }

    /// change window to fullscreen
    pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
        let _ = self
            .proxy
            .send_event(UserWindowEvent::Fullscreen(Box::new(fullscreen)));
    }

    /// set resizable state
    pub fn set_resizable(&self, resizable: bool) {
        let _ = self.proxy.send_event(UserWindowEvent::Resizable(resizable));
    }

    /// set the window always on top
    pub fn set_always_on_top(&self, top: bool) {
        let _ = self.proxy.send_event(UserWindowEvent::AlwaysOnTop(top));
    }

    // set cursor visible or not
    pub fn set_cursor_visible(&self, visible: bool) {
        let _ = self
            .proxy
            .send_event(UserWindowEvent::CursorVisible(visible));
    }

    // set cursor grab
    pub fn set_cursor_grab(&self, grab: bool) {
        let _ = self.proxy.send_event(UserWindowEvent::CursorGrab(grab));
    }

    /// set window title
    pub fn set_title(&self, title: &str) {
        let _ = self
            .proxy
            .send_event(UserWindowEvent::SetTitle(String::from(title)));
    }

    /// change window to borderless
    pub fn set_decorations(&self, decoration: bool) {
        let _ = self
            .proxy
            .send_event(UserWindowEvent::SetDecorations(decoration));
    }
}

/// use this function can get the `DesktopContext` context.
pub fn use_window(cx: &ScopeState) -> &Rc<DesktopContext> {
    cx.use_hook(|_| cx.consume_context::<DesktopContext>())
        .as_ref()
        .unwrap()
}
