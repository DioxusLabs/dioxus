use wry::application::event_loop::EventLoopProxy;

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
    pub fn minimize(&self, minimized: bool) {
        let _ = self.proxy.send_event(UserWindowEvent::Minimize(minimized));
    }

    /// set window maximize state
    pub fn maximize(&self, maximized: bool) {
        let _ = self.proxy.send_event(UserWindowEvent::Maximize(maximized));
    }

    /// close window
    pub fn close(&self) {
        let _ = self.proxy.send_event(UserWindowEvent::CloseWindow);
    }

}