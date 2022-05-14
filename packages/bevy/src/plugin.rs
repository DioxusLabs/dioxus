use crate::{
    context::UserEvent,
    event::{VirtualDomUpdated, WindowDragged},
    runner::runner,
    setting::DioxusSettings,
    window::DioxusWindows,
};
use bevy::{
    app::prelude::*,
    ecs::{event::Events, prelude::*},
    input::InputPlugin,
    log::error,
    window::{CreateWindow, WindowCreated, WindowPlugin, Windows},
};
use dioxus_core::Component as DioxusComponent;
use dioxus_desktop::{cfg::DesktopConfig, tao::event_loop::EventLoop};
use futures_intrusive::channel::shared::{channel, Sender};
use std::{fmt::Debug, marker::PhantomData};
use tokio::runtime::Runtime;

pub struct DioxusDesktopPlugin<CoreCommand, UICommand, Props = ()> {
    pub root: DioxusComponent<Props>,
    pub props: Props,
    core_cmd_type: PhantomData<CoreCommand>,
    ui_cmd_type: PhantomData<UICommand>,
}

impl<CoreCommand, UICommand, Props> Plugin for DioxusDesktopPlugin<CoreCommand, UICommand, Props>
where
    CoreCommand: 'static + Send + Sync + Clone + Debug,
    UICommand: 'static + Send + Sync + Clone + Debug,
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
        let settings = app
            .world
            .remove_non_send_resource::<DioxusSettings>()
            .unwrap_or_default();

        let event_loop = EventLoop::<UserEvent<CoreCommand>>::with_user_event();

        app.add_plugin(InputPlugin)
            .add_plugin(WindowPlugin::default())
            .add_event::<CoreCommand>()
            .add_event::<UICommand>()
            .add_event::<VirtualDomUpdated>()
            .add_event::<WindowDragged>()
            .insert_resource(core_tx)
            .insert_resource(core_rx)
            .insert_resource(ui_tx)
            .insert_resource(ui_rx)
            .insert_resource(runtime)
            .insert_resource(self.root)
            .insert_resource(self.props)
            .insert_resource(settings)
            .insert_non_send_resource(config)
            .init_non_send_resource::<DioxusWindows>()
            .set_runner(|app| runner::<CoreCommand, UICommand, Props>(app))
            .add_system_to_stage(CoreStage::Last, send_ui_commands::<UICommand>)
            .insert_non_send_resource(event_loop)
            .add_system(handle_virtual_dom_updated)
            .add_system(handle_window_dragged);

        Self::handle_initial_window_events(&mut app.world);
    }
}

impl<CoreCommand, UICommand, Props> DioxusDesktopPlugin<CoreCommand, UICommand, Props> {
    fn handle_initial_window_events(world: &mut World)
    where
        CoreCommand: 'static + Send + Sync + Clone + Debug,
        UICommand: 'static + Send + Sync + Clone + Debug,
        Props: 'static + Send + Sync + Copy,
    {
        let world = world.cell();
        let mut dioxus_windows = world.get_non_send_mut::<DioxusWindows>().unwrap();
        let mut bevy_windows = world.get_resource_mut::<Windows>().unwrap();
        let mut create_window_events = world.get_resource_mut::<Events<CreateWindow>>().unwrap();
        let mut window_created_events = world.get_resource_mut::<Events<WindowCreated>>().unwrap();

        for create_window_event in create_window_events.drain() {
            let window = dioxus_windows.create::<CoreCommand, UICommand, Props>(
                &world,
                create_window_event.id,
                &create_window_event.descriptor,
            );
            bevy_windows.add(window);
            window_created_events.send(WindowCreated {
                id: create_window_event.id,
            });
        }
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

fn handle_virtual_dom_updated(
    mut events: EventReader<VirtualDomUpdated>,
    mut windows: NonSendMut<DioxusWindows>,
) {
    for e in events.iter() {
        let window = windows.get_mut(e.window_id).unwrap();
        window.try_load_ready_webview();
    }
}

fn handle_window_dragged(
    mut events: EventReader<WindowDragged>,
    mut windows: NonSendMut<DioxusWindows>,
) {
    for e in events.iter() {
        let window = windows.get(e.window_id).unwrap();
        let tao_window = window.tao_window();

        tao_window
            .fullscreen()
            .is_none()
            .then(|| tao_window.drag_window());
    }
}
