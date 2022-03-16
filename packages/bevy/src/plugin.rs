use crate::{context::BevyDesktopContext, event::CustomUserEvent, runner::runner};
use bevy::{
    app::{App, CoreStage, Plugin},
    ecs::{event::EventReader, system::Res},
    input::InputPlugin,
    log::error,
};
use dioxus_core::*;
use dioxus_desktop::{
    controller::DesktopController, desktop_context::UserEvent, tao::event_loop::EventLoop,
};
use futures_channel::mpsc::unbounded;
use std::{fmt::Debug, marker::PhantomData};
use tokio::sync::broadcast::{channel, Sender};

pub struct DioxusDesktopPlugin<CoreCommand, UICommand, Props = ()> {
    root: Component<Props>,
    props: Props,
    core_cmd_type: PhantomData<CoreCommand>,
    ui_cmd_type: PhantomData<UICommand>,
}

impl<CoreCommand, UICommand, Props> Plugin for DioxusDesktopPlugin<CoreCommand, UICommand, Props>
where
    CoreCommand: 'static + Send + Sync + Clone + Debug,
    UICommand: 'static + Send + Sync + Clone + Copy,
    Props: 'static + Send + Sync + Copy,
{
    fn build(&self, app: &mut App) {
        let event_loop = EventLoop::<UserEvent<CustomUserEvent<CoreCommand>>>::with_user_event();

        let (core_tx, core_rx) = unbounded::<CoreCommand>();
        let (ui_tx, _) = channel::<UICommand>(8);

        let proxy = event_loop.create_proxy();

        let desktop = DesktopController::new_on_tokio(
            self.root,
            self.props,
            proxy.clone(),
            BevyDesktopContext::<CustomUserEvent<CoreCommand>, CoreCommand, UICommand>::new(
                proxy,
                (core_tx, ui_tx.clone()),
            ),
        );

        app.add_plugin(InputPlugin)
            .add_event::<CoreCommand>()
            .add_event::<UICommand>()
            .insert_non_send_resource(event_loop)
            .insert_non_send_resource(desktop)
            .insert_resource(ui_tx)
            .insert_resource(core_rx)
            .set_runner(|app| runner::<CoreCommand, UICommand>(app))
            .add_system_to_stage(CoreStage::Last, send_ui_commands::<UICommand>);
    }
}

impl<CoreCommand, UICommand, Props> DioxusDesktopPlugin<CoreCommand, UICommand, Props> {
    pub fn new(root: Component<Props>, props: Props) -> Self {
        Self {
            root,
            props,
            core_cmd_type: PhantomData,
            ui_cmd_type: PhantomData,
        }
    }
}

fn send_ui_commands<UICommand>(mut events: EventReader<UICommand>, tx: Res<Sender<UICommand>>)
where
    UICommand: 'static + Send + Sync + Copy,
{
    for e in events.iter() {
        if let Err(err) = tx.send(*e) {
            error!("{err}");
        };
    }
}
