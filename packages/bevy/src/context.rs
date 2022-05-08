use dioxus_desktop::desktop_context::{ProxyType, WindowController};
use futures_intrusive::channel::shared::{Receiver, Sender};
use std::fmt::Debug;

#[derive(Clone)]
pub struct BevyDesktopContext<
    CustomUserEvent: Debug + 'static,
    CoreCommand: Debug + 'static + Clone,
    UICommand: 'static + Clone,
> {
    proxy: ProxyType<CustomUserEvent>,
    channel: (Sender<CoreCommand>, Receiver<UICommand>),
}

impl<CustomUserEvent, CoreCommand, UICommand> WindowController<CustomUserEvent>
    for BevyDesktopContext<CustomUserEvent, CoreCommand, UICommand>
where
    CustomUserEvent: Debug + Clone,
    CoreCommand: Debug + Clone,
    UICommand: Debug + Clone,
{
    fn get_proxy(&self) -> ProxyType<CustomUserEvent> {
        self.proxy.clone()
    }
}

impl<CustomUserEvent, CoreCommand, UICommand>
    BevyDesktopContext<CustomUserEvent, CoreCommand, UICommand>
where
    CustomUserEvent: Debug,
    CoreCommand: Debug + Clone,
    UICommand: Clone,
{
    pub fn new(
        proxy: ProxyType<CustomUserEvent>,
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

    // TODO: /// run (evaluate) a script in the WebView context
    // pub fn eval(&self, script: impl std::string::ToString) {
    //     let _ = self
    //         .proxy
    //         .send_event(UserEvent::WindowEvent(Eval(script.to_string())));
    // }
}
