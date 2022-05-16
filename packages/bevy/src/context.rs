use crate::event::WebKeyboardEvent;
use dioxus_desktop::{
    desktop_context::UserWindowEvent::{self, *},
    tao::event_loop::EventLoopProxy,
};
use futures_intrusive::channel::shared::{Receiver, Sender};
use std::fmt::Debug;

pub type ProxyType<CoreCommand> = EventLoopProxy<UserEvent<CoreCommand>>;

#[derive(Debug)]
pub enum UserEvent<CoreCommand: Debug> {
    WindowEvent(UserWindowEvent),
    CoreCommand(CoreCommand),
    KeyboardEvent(WebKeyboardEvent),
}

#[derive(Clone)]
pub struct DesktopContext<CoreCommand: Debug + 'static + Clone, UICommand: 'static + Clone> {
    proxy: ProxyType<CoreCommand>,
    channel: (Sender<CoreCommand>, Receiver<UICommand>),
}

impl<CoreCommand, UICommand> DesktopContext<CoreCommand, UICommand>
where
    CoreCommand: Debug + Clone,
    UICommand: Debug + Clone,
{
    pub fn new(
        proxy: ProxyType<CoreCommand>,
        channel: (Sender<CoreCommand>, Receiver<UICommand>),
    ) -> Self {
        Self { proxy, channel }
    }

    pub fn receiver(&self) -> Receiver<UICommand> {
        self.channel.1.clone()
    }

    pub fn send(&self, cmd: CoreCommand) {
        self.channel
            .0
            .try_send(cmd)
            .expect("Failed to send CoreCommand");
    }

    pub fn drag(&self) {
        let _ = self.proxy.send_event(UserEvent::WindowEvent(DragWindow));
    }

    pub fn set_minimized(&self, minimized: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(Minimize(minimized)));
    }

    pub fn set_maximized(&self, maximized: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(Maximize(maximized)));
    }

    pub fn toggle_maximized(&self) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(MaximizeToggle));
    }

    pub fn set_visible(&self, visible: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(Visible(visible)));
    }

    pub fn close(&self) {
        let _ = self.proxy.send_event(UserEvent::WindowEvent(CloseWindow));
    }

    pub fn focus(&self) {
        let _ = self.proxy.send_event(UserEvent::WindowEvent(FocusWindow));
    }

    pub fn set_fullscreen(&self, fullscreen: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(Fullscreen(fullscreen)));
    }

    pub fn set_resizable(&self, resizable: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(Resizable(resizable)));
    }

    pub fn set_always_on_top(&self, top: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(AlwaysOnTop(top)));
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(CursorVisible(visible)));
    }

    pub fn set_cursor_grab(&self, grab: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(CursorGrab(grab)));
    }

    pub fn set_title(&self, title: &str) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(SetTitle(String::from(title))));
    }

    pub fn set_decorations(&self, decoration: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(SetDecorations(decoration)));
    }

    pub fn devtool(&self) {
        let _ = self.proxy.send_event(UserEvent::WindowEvent(DevTool));
    }

    pub fn eval(&self, script: impl std::string::ToString) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(Eval(script.to_string())));
    }
}
