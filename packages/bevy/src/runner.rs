use crate::{
    context::UserEvent,
    setting::{DioxusSettings, UpdateMode},
    window::DioxusWindows,
};
use bevy::{
    app::{App, AppExit},
    ecs::{
        event::{Events, ManualEventReader},
        world::World,
    },
    input::{keyboard::KeyboardInput, mouse::MouseMotion},
    log::{info, warn},
    math::Vec2,
    utils::Instant,
    window::{
        CreateWindow, ReceivedCharacter, RequestRedraw, WindowCloseRequested, WindowCreated,
        Windows,
    },
};
use dioxus_desktop::{
    desktop_context::UserWindowEvent::*,
    tao::{
        event::{DeviceEvent, Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
        window::Fullscreen as WryFullscreen,
    },
};
use futures_intrusive::channel::shared::Receiver;
use std::fmt::Debug;
use tokio::runtime::Runtime;

pub fn runner<CoreCommand, UICommand, Props>(mut app: App)
where
    CoreCommand: 'static + Send + Sync + Clone + Debug,
    UICommand: 'static + Send + Sync + Clone + Debug,
    Props: 'static + Send + Sync + Copy,
{
    let event_loop = app
        .world
        .remove_non_send_resource::<EventLoop<UserEvent<CoreCommand>>>()
        .unwrap();
    let core_rx = app
        .world
        .remove_resource::<Receiver<CoreCommand>>()
        .unwrap();
    let runtime = app.world.get_resource::<Runtime>().unwrap();
    let proxy = event_loop.create_proxy();

    let mut tao_state = TaoPersistentState::default();

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
            let mut windows = app
                .world
                .get_non_send_resource_mut::<DioxusWindows>()
                .expect("Insert DioxusWindows as non send resource");

            match event {
                Event::NewEvents(start) => {
                    let dioxus_settings = app.world.resource::<DioxusSettings>();
                    let windows = app.world.resource::<Windows>();
                    let focused = windows.iter().any(|w| w.is_focused());
                    let auto_timeout_reached =
                        matches!(start, StartCause::ResumeTimeReached { .. });
                    let now = Instant::now();
                    let manual_timeout_reached = match dioxus_settings.update_mode(focused) {
                        UpdateMode::Continuous => false,
                        UpdateMode::Reactive { max_wait }
                        | UpdateMode::ReactiveLowPower { max_wait } => {
                            now.duration_since(tao_state.last_update) >= *max_wait
                        }
                    };
                    tao_state.low_power_event = false;
                    tao_state.timeout_reached = auto_timeout_reached || manual_timeout_reached;
                }
                Event::WindowEvent {
                    event,
                    window_id: tao_window_id,
                    ..
                } => {
                    let world = app.world.cell();
                    let dioxus_windows = world.get_non_send_mut::<DioxusWindows>().unwrap();
                    let mut windows = world.get_resource_mut::<Windows>().unwrap();
                    let window_id =
                        if let Some(window_id) = dioxus_windows.get_window_id(tao_window_id) {
                            window_id
                        } else {
                            warn!(
                                "Skipped event for unknown winit Window Id {:?}",
                                tao_window_id
                            );
                            return;
                        };

                    let _window = if let Some(window) = windows.get_mut(window_id) {
                        window
                    } else {
                        info!("Skipped event for closed window: {:?}", window_id);
                        return;
                    };
                    tao_state.low_power_event = true;

                    match event {
                        WindowEvent::CloseRequested => {
                            let mut window_close_requested_events = world
                                .get_resource_mut::<Events<WindowCloseRequested>>()
                                .unwrap();
                            window_close_requested_events
                                .send(WindowCloseRequested { id: window_id });
                        }
                        WindowEvent::Destroyed { .. } => {
                            // windows.remove(&tao_window_id, control_flow)
                        }
                        WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                            // windows.resize(&tao_window_id);
                        }
                        _ => {}
                    }
                }
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

                            SetZoomLevel(scale_factor) => webview.zoom(scale_factor),

                            Print => {
                                if let Err(e) = webview.print() {
                                    // we can't panic this error.
                                    log::warn!("Open print modal failed: {e}");
                                }
                            }
                            DevTool => webview.open_devtools(),

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
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta, .. },
                    ..
                } => {
                    let mut mouse_motion_events = app.world.resource_mut::<Events<MouseMotion>>();
                    mouse_motion_events.send(MouseMotion {
                        delta: Vec2::new(delta.0 as f32, delta.1 as f32),
                    });
                }
                Event::Suspended => {
                    tao_state.active = false;
                }
                Event::Resumed => {
                    tao_state.active = true;
                }
                Event::MainEventsCleared => {
                    handle_create_window_events::<CoreCommand, UICommand, Props>(&mut app.world);
                    let dioxus_settings = app.world.resource::<DioxusSettings>();
                    let update = if tao_state.active {
                        let windows = app.world.resource::<Windows>();
                        let focused = windows.iter().any(|w| w.is_focused());
                        match dioxus_settings.update_mode(focused) {
                            UpdateMode::Continuous | UpdateMode::Reactive { .. } => true,
                            UpdateMode::ReactiveLowPower { .. } => {
                                tao_state.low_power_event
                                    || tao_state.redraw_request_sent
                                    || tao_state.timeout_reached
                            }
                        }
                    } else {
                        false
                    };

                    if update {
                        tao_state.last_update = Instant::now();
                        app.update();
                    }
                }
                Event::RedrawEventsCleared => {
                    {
                        let dioxus_settings = app.world.resource::<DioxusSettings>();
                        let windows = app.world.non_send_resource::<Windows>();
                        let focused = windows.iter().any(|w| w.is_focused());
                        let now = Instant::now();
                        use UpdateMode::*;
                        *control_flow = match dioxus_settings.update_mode(focused) {
                            Continuous => ControlFlow::Poll,
                            Reactive { max_wait } | ReactiveLowPower { max_wait } => {
                                ControlFlow::WaitUntil(now + *max_wait)
                            }
                        };
                    }
                    let mut redraw = false;
                    if let Some(app_redraw_events) =
                        app.world.get_resource::<Events<RequestRedraw>>()
                    {
                        let mut redraw_event_reader = ManualEventReader::<RequestRedraw>::default();
                        if redraw_event_reader.iter(app_redraw_events).last().is_some() {
                            *control_flow = ControlFlow::Poll;
                            redraw = true;
                        }
                    }

                    if let Some(app_exit_events) = app.world.get_resource::<Events<AppExit>>() {
                        let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
                        if app_exit_event_reader.iter(app_exit_events).last().is_some() {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    tao_state.redraw_request_sent = redraw;
                }
                _ => {}
            }
        },
    );
}

fn handle_create_window_events<CoreCommand, UICommand, Props>(world: &mut World)
where
    CoreCommand: 'static + Send + Sync + Clone + Debug,
    UICommand: 'static + Send + Sync + Clone + Debug,
    Props: 'static + Send + Sync + Copy,
{
    let world = world.cell();
    let mut dioxus_windows = world.get_non_send_mut::<DioxusWindows>().unwrap();
    let mut windows = world.get_resource_mut::<Windows>().unwrap();
    let create_window_events = world.get_resource::<Events<CreateWindow>>().unwrap();
    let mut create_window_events_reader = ManualEventReader::<CreateWindow>::default();
    let mut window_created_events = world.get_resource_mut::<Events<WindowCreated>>().unwrap();

    for create_window_event in create_window_events_reader.iter(&create_window_events) {
        let window = dioxus_windows.create::<CoreCommand, UICommand, Props>(
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

/// Stores state that must persist between frames.
struct TaoPersistentState {
    /// Tracks whether or not the application is active or suspended.
    active: bool,
    /// Tracks whether or not an event has occurred this frame that would trigger an update in low
    /// power mode. Should be reset at the end of every frame.
    low_power_event: bool,
    /// Tracks whether the event loop was started this frame because of a redraw request.
    redraw_request_sent: bool,
    /// Tracks if the event loop was started this frame because of a `WaitUntil` timeout.
    timeout_reached: bool,
    last_update: Instant,
}
impl Default for TaoPersistentState {
    fn default() -> Self {
        Self {
            active: true,
            low_power_event: false,
            redraw_request_sent: false,
            timeout_reached: false,
            last_update: Instant::now(),
        }
    }
}
