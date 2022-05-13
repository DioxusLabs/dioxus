use crate::{context::UserEvent, window::DioxusWindows};
use bevy::{
    app::{App, AppExit},
    ecs::event::{Events, ManualEventReader},
    input::keyboard::KeyboardInput,
    window::ReceivedCharacter,
};
use dioxus_desktop::{
    desktop_context::UserWindowEvent::*,
    tao::{
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
        window::Fullscreen as WryFullscreen,
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
        .remove_non_send_resource::<EventLoop<UserEvent<CoreCommand>>>()
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
                .send_event(UserEvent::CoreCommand(cmd))
                .unwrap();
        }
    });

    event_loop.run(
        move |event: Event<UserEvent<CoreCommand>>,
              _event_loop: &EventLoopWindowTarget<UserEvent<CoreCommand>>,
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

            let mut windows = app
                .world
                .get_non_send_resource_mut::<DioxusWindows>()
                .expect("Insert DioxusWindows as non send resource");

            match event {
                Event::NewEvents(StartCause::Init) => {}
                Event::WindowEvent {
                    event, window_id, ..
                } => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Destroyed { .. } => windows.remove(&window_id, control_flow),
                    WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                        windows.resize(&window_id);
                    }
                    _ => {}
                },
                Event::UserEvent(user_event) => match user_event {
                    UserEvent::WindowEvent(user_window_event) => {
                        // currently dioxus-desktop supports a single window only,
                        // so we can grab the only webview from the map;
                        let window = windows.get_one().unwrap();
                        let webview = &window.webview;
                        let tao_window = webview.window();

                        match user_window_event {
                            Update => window.try_load_ready_webview(),
                            CloseWindow => *control_flow = ControlFlow::Exit,
                            DragWindow => {
                                // if the drag_window has any errors, we don't do anything
                                tao_window
                                    .fullscreen()
                                    .is_none()
                                    .then(|| tao_window.drag_window());
                            }
                            Visible(state) => tao_window.set_visible(state),
                            Minimize(state) => tao_window.set_minimized(state),
                            Maximize(state) => tao_window.set_maximized(state),
                            MaximizeToggle => tao_window.set_maximized(!tao_window.is_maximized()),
                            Fullscreen(state) => {
                                if let Some(handle) = tao_window.current_monitor() {
                                    tao_window.set_fullscreen(
                                        state.then(|| WryFullscreen::Borderless(Some(handle))),
                                    );
                                }
                            }
                            FocusWindow => tao_window.set_focus(),
                            Resizable(state) => tao_window.set_resizable(state),
                            AlwaysOnTop(state) => tao_window.set_always_on_top(state),

                            CursorVisible(state) => tao_window.set_cursor_visible(state),
                            CursorGrab(state) => {
                                let _ = tao_window.set_cursor_grab(state);
                            }

                            SetTitle(content) => tao_window.set_title(&content),
                            SetDecorations(state) => tao_window.set_decorations(state),

                            DevTool => webview.devtool(),

                            Eval(code) => webview
                                .evaluate_script(code.as_str())
                                .expect("eval shouldn't panic"),
                        };
                    }
                    UserEvent::CoreCommand(cmd) => {
                        let mut events = app
                            .world
                            .get_resource_mut::<Events<CoreCommand>>()
                            .expect("Provide CoreCommand event to bevy");
                        events.send(cmd);

                        app.update();
                    }
                    UserEvent::KeyboardEvent(event) => {
                        let mut keyboard_input_events = app
                            .world
                            .get_resource_mut::<Events<KeyboardInput>>()
                            .unwrap();
                        keyboard_input_events.send(event.to_input());

                        match event.try_to_char() {
                            Some(c) => {
                                let mut received_character_events = app
                                    .world
                                    .get_resource_mut::<Events<ReceivedCharacter>>()
                                    .unwrap();
                                received_character_events.send(c);
                            }
                            None => {}
                        }

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
