use crate::{
    context::BevyDesktopContext, event::CustomUserEvent, runner::runner, window::DioxusWindows,
};
use bevy::{
    app::prelude::*,
    ecs::{event::Events, prelude::*},
    input::InputPlugin,
    log::error,
    window::{CreateWindow, WindowCreated, WindowPlugin, Windows},
};
use dioxus_core::Component as DioxusComponent;
use dioxus_desktop::{
    cfg::DesktopConfig, controller::DesktopController, desktop_context::UserEvent,
    tao::event_loop::EventLoop,
};
use futures_intrusive::channel::shared::{channel, Sender};
use std::{fmt::Debug, marker::PhantomData};
use tokio::runtime::Runtime;

pub struct DioxusDesktopPlugin<CoreCommand, UICommand, Props = ()> {
    root: DioxusComponent<Props>,
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

        let (core_tx, core_rx) = channel::<CoreCommand>(8);
        let (ui_tx, ui_rx) = channel::<UICommand>(8);
        let config = app
            .world
            .remove_non_send_resource::<DesktopConfig>()
            .unwrap_or_default();

        let event_loop = EventLoop::<UserEvent<CustomUserEvent<CoreCommand>>>::with_user_event();
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
            .add_plugin(WindowPlugin::default())
            .add_event::<CoreCommand>()
            .add_event::<UICommand>()
            .insert_resource(ui_tx)
            .insert_resource(core_rx)
            .insert_resource(runtime)
            .insert_non_send_resource(config)
            .init_non_send_resource::<DioxusWindows>()
            .set_runner(|app| runner::<CoreCommand, UICommand>(app))
            .add_system_to_stage(CoreStage::Last, send_ui_commands::<UICommand>)
            .insert_non_send_resource(event_loop)
            .insert_non_send_resource(desktop);

        handle_initial_window_events::<CoreCommand>(&mut app.world);
    }
}

impl<CoreCommand, UICommand, Props> DioxusDesktopPlugin<CoreCommand, UICommand, Props> {
    pub fn new(root: DioxusComponent<Props>, props: Props) -> Self {
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

fn handle_initial_window_events<CoreCommand>(world: &mut World)
where
    CoreCommand: 'static + Debug,
{
    let world = world.cell();
    let mut winit_windows = world.get_non_send_mut::<DioxusWindows>().unwrap();
    let mut windows = world.get_resource_mut::<Windows>().unwrap();
    let mut create_window_events = world.get_resource_mut::<Events<CreateWindow>>().unwrap();
    let mut window_created_events = world.get_resource_mut::<Events<WindowCreated>>().unwrap();

    for create_window_event in create_window_events.drain() {
        let window = winit_windows.create_window::<CoreCommand>(
            &world,
            create_window_event.id,
            &create_window_event.descriptor,
        );
        windows.add(window);
        window_created_events.send(WindowCreated {
            id: create_window_event.id,
        });
    }
}
