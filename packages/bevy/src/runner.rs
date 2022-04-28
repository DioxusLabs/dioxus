use crate::event::CustomUserEvent;
use bevy::{
    app::{App, AppExit},
    ecs::event::{Events, ManualEventReader},
    input::keyboard::KeyboardInput,
};
use dioxus_desktop::{
    controller::DesktopController,
    desktop_context::{user_window_event_handler, UserEvent},
    tao::{
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    },
};
use futures_intrusive::channel::shared::Receiver;
use std::fmt::Debug;
use tokio::runtime::Runtime;

pub fn runner<CoreCommand, UICommand>(mut app: App)
where
    CoreCommand: 'static + Send + Sync + Debug,
    UICommand: 'static,
{
    let event_loop = app
        .world
        .remove_non_send_resource::<EventLoop<UserEvent<CustomUserEvent<CoreCommand>>>>()
        .expect("Insert EventLoop as non send resource");

    let core_rx = app
        .world
        .remove_resource::<Receiver<CoreCommand>>()
        .expect("Failed to retrieve CoreCommand receiver resource");

    let runtime = app
        .world
        .get_resource::<Runtime>()
        .expect("Failed to retrieve async runtime");
    let proxy = event_loop.create_proxy();

    runtime.spawn(async move {
        while let Some(cmd) = core_rx.receive().await {
            proxy
                .clone()
                .send_event(UserEvent::CustomEvent(CustomUserEvent::CoreCommand(cmd)))
                .unwrap();
        }
    });

    event_loop.run(
        move |window_event: Event<UserEvent<CustomUserEvent<CoreCommand>>>,
              _event_loop: &EventLoopWindowTarget<UserEvent<CustomUserEvent<CoreCommand>>>,
              control_flow: &mut ControlFlow| {
            *control_flow = ControlFlow::Wait;

            let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
            if let Some(app_exit_events) = app.world.get_resource_mut::<Events<AppExit>>() {
                if app_exit_event_reader
                    .iter(&app_exit_events)
                    .next_back()
                    .is_some()
                {
                    *control_flow = ControlFlow::Exit;
                }
            }
            let mut desktop = app
                .world
                .get_non_send_resource_mut::<DesktopController>()
                .expect("Insert DesktopController as non send resource");

            match window_event {
                Event::NewEvents(StartCause::Init) => {}

                Event::WindowEvent {
                    event, window_id, ..
                } => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Destroyed { .. } => desktop.close_window(window_id, control_flow),

                    WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                        if let Some(view) = desktop.webviews.get_mut(&window_id) {
                            let _ = view.resize();
                        }
                    }
                    _ => {}
                },

                Event::UserEvent(user_event) => match user_event {
                    UserEvent::WindowEvent(e) => {
                        user_window_event_handler(e, &mut desktop, control_flow);
                    }
                    UserEvent::CustomEvent(e) => {
                        match e {
                            CustomUserEvent::CoreCommand(cmd) => {
                                let mut events = app
                                    .world
                                    .get_resource_mut::<Events<CoreCommand>>()
                                    .expect("Provide CoreCommand event to bevy");
                                events.send(cmd);
                                app.update();
                            }
                            CustomUserEvent::KeyboardInput(input) => {
                                let mut events = app
                                    .world
                                    .get_resource_mut::<Events<KeyboardInput>>()
                                    .unwrap();
                                events.send(input);
                            }
                        };
                        app.update();
                    }
                },
                Event::MainEventsCleared => {}
                Event::Resumed => {}
                Event::Suspended => {}
                Event::LoopDestroyed => {}
                Event::RedrawRequested(_id) => {}
                _ => {}
            }
        },
    );
}
