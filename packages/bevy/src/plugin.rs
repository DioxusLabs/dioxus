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
use futures_intrusive::channel::shared::{channel, Sender};
use std::{fmt::Debug, marker::PhantomData};
use tokio::runtime::Runtime;

pub struct DioxusDesktopPlugin<CoreCommand, UICommand, Props = ()> {
    root: Component<Props>,
    props: Props,
    core_cmd_type: PhantomData<CoreCommand>,
    ui_cmd_type: PhantomData<UICommand>,
}

impl<CoreCommand, UICommand, Props> Plugin for DioxusDesktopPlugin<CoreCommand, UICommand, Props>
where
    CoreCommand: 'static + Send + Sync + Clone + Debug,
    UICommand: 'static + Send + Sync + Clone,
    Props: 'static + Send + Sync + Copy,
{
    fn build(&self, app: &mut App) {
        let runtime = Runtime::new().unwrap();
        let event_loop = EventLoop::<UserEvent<CustomUserEvent<CoreCommand>>>::with_user_event();

        let (core_tx, core_rx) = channel::<CoreCommand>(8);
        let (ui_tx, ui_rx) = channel::<UICommand>(8);

        let proxy = event_loop.create_proxy();

        let desktop = DesktopController::new_on_tokio(
            self.root,
            self.props,
            proxy.clone(),
            BevyDesktopContext::<CustomUserEvent<CoreCommand>, CoreCommand, UICommand>::new(
                proxy,
                (core_tx, ui_rx),
            ),
        );

        app.add_plugin(InputPlugin)
            .add_event::<CoreCommand>()
            .add_event::<UICommand>()
            .insert_non_send_resource(event_loop)
            .insert_non_send_resource(desktop)
            .insert_resource(ui_tx)
            .insert_resource(core_rx)
            .insert_resource(runtime)
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
    UICommand: 'static + Send + Sync + Clone,
{
    for e in events.iter() {
        if let Err(_) = tx.try_send(e.clone()) {
            error!("Failed to send UICommand");
        };
    }
}
