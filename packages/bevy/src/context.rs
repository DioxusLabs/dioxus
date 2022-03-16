use dioxus_desktop::desktop_context::{ProxyType, WindowController};
use futures_channel::mpsc::{TrySendError, UnboundedSender};
use std::fmt::Debug;
use tokio::sync::broadcast::{Receiver, Sender};

#[derive(Clone)]
pub struct BevyDesktopContext<
    CustomUserEvent: Debug + 'static,
    CoreCommand: Debug + 'static + Clone,
    UICommand,
> {
    proxy: ProxyType<CustomUserEvent>,
    channel: (UnboundedSender<CoreCommand>, Sender<UICommand>),
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
{
    pub fn new(
        proxy: ProxyType<CustomUserEvent>,
        channel: (UnboundedSender<CoreCommand>, Sender<UICommand>),
    ) -> Self {
        Self { proxy, channel }
    }

    pub fn receiver(&self) -> Receiver<UICommand> {
        self.channel.1.subscribe()
    }

    pub fn send(&self, cmd: CoreCommand) -> Result<(), TrySendError<CoreCommand>> {
        self.channel.0.unbounded_send(cmd)
    }
}
