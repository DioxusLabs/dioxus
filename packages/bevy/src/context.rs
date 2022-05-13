use crate::event::WebKeyboardEvent;
use dioxus_desktop::{
    desktop_context::UserWindowEvent::{self, *},
    tao::event_loop::EventLoopProxy,
};
use futures_intrusive::channel::shared::{Receiver, Sender};
use std::fmt::Debug;

type ProxyType<CoreCommand> = EventLoopProxy<UserEvent<CoreCommand>>;

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

    /// trigger the drag-window event
    ///
    /// Moves the window with the left mouse button until the button is released.
    ///
    /// you need use it in `onmousedown` event:
    /// ```rust
    /// onmousedown: move |_| { desktop.drag_window(); }
    /// ```
    pub fn drag(&self) {
        let _ = self.proxy.send_event(UserEvent::WindowEvent(DragWindow));
    }

    /// set window minimize state
    pub fn set_minimized(&self, minimized: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(Minimize(minimized)));
    }

    /// set window maximize state
    pub fn set_maximized(&self, maximized: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(Maximize(maximized)));
    }

    /// toggle window maximize state
    pub fn toggle_maximized(&self) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(MaximizeToggle));
    }

    /// set window visible or not
    pub fn set_visible(&self, visible: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(Visible(visible)));
    }

    /// close window
    pub fn close(&self) {
        let _ = self.proxy.send_event(UserEvent::WindowEvent(CloseWindow));
    }

    /// set window to focus
    pub fn focus(&self) {
        let _ = self.proxy.send_event(UserEvent::WindowEvent(FocusWindow));
    }

    /// change window to fullscreen
    pub fn set_fullscreen(&self, fullscreen: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(Fullscreen(fullscreen)));
    }

    /// set resizable state
    pub fn set_resizable(&self, resizable: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(Resizable(resizable)));
    }

    /// set the window always on top
    pub fn set_always_on_top(&self, top: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(AlwaysOnTop(top)));
    }

    /// set cursor visible or not
    pub fn set_cursor_visible(&self, visible: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(CursorVisible(visible)));
    }

    /// set cursor grab
    pub fn set_cursor_grab(&self, grab: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(CursorGrab(grab)));
    }

    /// set window title
    pub fn set_title(&self, title: &str) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(SetTitle(String::from(title))));
    }

    /// change window to borderless
    pub fn set_decorations(&self, decoration: bool) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(SetDecorations(decoration)));
    }

    /// opens DevTool window
    pub fn devtool(&self) {
        let _ = self.proxy.send_event(UserEvent::WindowEvent(DevTool));
    }

    /// run (evaluate) a script in the WebView context
    pub fn eval(&self, script: impl std::string::ToString) {
        let _ = self
            .proxy
            .send_event(UserEvent::WindowEvent(Eval(script.to_string())));
    }
}
