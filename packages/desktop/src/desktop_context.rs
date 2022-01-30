use wry::application::event_loop::EventLoopProxy;

use crate::UserWindowEvent;

type ProxyType = EventLoopProxy<UserWindowEvent>;

#[derive(Clone)]
pub struct DesktopContext {
    proxy: ProxyType,
}

impl DesktopContext {
    pub fn new(proxy: ProxyType) -> Self {
        Self { proxy }
    }

    pub fn drag_window(&self) {
        let _ = self.proxy.send_event(UserWindowEvent::DragWindow);
    }

    pub fn minimized(&self, minimized: bool) {
        let _ = self.proxy.send_event(UserWindowEvent::Minimized(minimized));
    }

    pub fn maximized(&self, maximized: bool) {
        let _ = self.proxy.send_event(UserWindowEvent::Maximized(maximized));
    }
}