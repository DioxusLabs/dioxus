use crate::{
    context::UserEvent,
    event::{DomUpdated, VisibleUpdated, WindowDragged, WindowMaximized, WindowMinimized},
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
    math::{ivec2, Vec2},
    utils::Instant,
    window::{
        CreateWindow, FileDragAndDrop, ReceivedCharacter, RequestRedraw,
        WindowBackendScaleFactorChanged, WindowCloseRequested, WindowCreated, WindowFocused,
        WindowId, WindowMoved, WindowResized, WindowScaleFactorChanged, Windows,
    },
};
use dioxus_desktop::{
    desktop_context::UserWindowEvent,
    tao::{
        dpi::LogicalSize,
        event::{DeviceEvent, Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
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
            // let mut windows = app
            //     .world
            //     .get_non_send_resource_mut::<DioxusWindows>()
            //     .expect("Insert DioxusWindows as non send resource");

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

                    let window = if let Some(window) = windows.get_mut(window_id) {
                        window
                    } else {
                        info!("Skipped event for closed window: {:?}", window_id);
                        return;
                    };
                    tao_state.low_power_event = true;

                    match event {
                        WindowEvent::Resized(size) => {
                            window.update_actual_size_from_backend(size.width, size.height);
                            let mut resize_events =
                                world.get_resource_mut::<Events<WindowResized>>().unwrap();
                            resize_events.send(WindowResized {
                                id: window_id,
                                width: window.width(),
                                height: window.height(),
                            });
                        }
                        WindowEvent::CloseRequested => {
                            let mut window_close_requested_events = world
                                .get_resource_mut::<Events<WindowCloseRequested>>()
                                .unwrap();
                            window_close_requested_events
                                .send(WindowCloseRequested { id: window_id });
                        }
                        // No event emitted. probably webview interrupts window underneath
                        // WindowEvent::KeyboardInput { event, .. } => {
                        //     println!("event: {:?}", event);
                        // }
                        // WindowEvent::CursorMoved { device_id, .. } => {
                        //     println!("device_id: {:?}", device_id);
                        // }
                        // WindowEvent::CursorEntered { device_id } => {
                        //     println!("device_id: {:?}", device_id);
                        // }
                        // WindowEvent::CursorLeft { device_id } => {
                        //     println!("device_id: {:?}", device_id);
                        // }
                        // WindowEvent::MouseInput { device_id, .. } => {
                        //     println!("device_id: {:?}", device_id);
                        // }
                        // WindowEvent::MouseWheel { device_id, .. } => {
                        //     println!("device_id: {:?}", device_id);
                        // }
                        // WindowEvent::Touch(touch) => {
                        //     println!("touch: {:?}", touch);
                        // }
                        // it doesn't event exist in tao or wry but in winit
                        // WindowEvent::ReceivedCharacter(char) => {
                        //     println!("char: {}", char);
                        // }
                        WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            new_inner_size,
                        } => {
                            let mut backend_scale_factor_change_events = world
                                .get_resource_mut::<Events<WindowBackendScaleFactorChanged>>()
                                .unwrap();
                            backend_scale_factor_change_events.send(
                                WindowBackendScaleFactorChanged {
                                    id: window_id,
                                    scale_factor,
                                },
                            );
                            let prior_factor = window.scale_factor();
                            window.update_scale_factor_from_backend(scale_factor);
                            let new_factor = window.scale_factor();
                            if let Some(forced_factor) = window.scale_factor_override() {
                                *new_inner_size = LogicalSize::new(
                                    window.requested_width(),
                                    window.requested_height(),
                                )
                                .to_physical::<u32>(forced_factor);
                            } else if approx::relative_ne!(new_factor, prior_factor) {
                                let mut scale_factor_change_events = world
                                    .get_resource_mut::<Events<WindowScaleFactorChanged>>()
                                    .unwrap();

                                scale_factor_change_events.send(WindowScaleFactorChanged {
                                    id: window_id,
                                    scale_factor,
                                });
                            }

                            let new_logical_width = new_inner_size.width as f64 / new_factor;
                            let new_logical_height = new_inner_size.height as f64 / new_factor;
                            if approx::relative_ne!(window.width() as f64, new_logical_width)
                                || approx::relative_ne!(window.height() as f64, new_logical_height)
                            {
                                let mut resize_events =
                                    world.get_resource_mut::<Events<WindowResized>>().unwrap();
                                resize_events.send(WindowResized {
                                    id: window_id,
                                    width: new_logical_width as f32,
                                    height: new_logical_height as f32,
                                });
                            }
                            window.update_actual_size_from_backend(
                                new_inner_size.width,
                                new_inner_size.height,
                            );
                        }
                        WindowEvent::Focused(focused) => {
                            window.update_focused_status_from_backend(focused);
                            let mut focused_events =
                                world.get_resource_mut::<Events<WindowFocused>>().unwrap();
                            focused_events.send(WindowFocused {
                                id: window_id,
                                focused,
                            });
                        }
                        WindowEvent::DroppedFile(path_buf) => {
                            let mut events =
                                world.get_resource_mut::<Events<FileDragAndDrop>>().unwrap();
                            events.send(FileDragAndDrop::DroppedFile {
                                id: window_id,
                                path_buf,
                            });
                        }
                        WindowEvent::HoveredFile(path_buf) => {
                            let mut events =
                                world.get_resource_mut::<Events<FileDragAndDrop>>().unwrap();
                            events.send(FileDragAndDrop::HoveredFile {
                                id: window_id,
                                path_buf,
                            });
                        }
                        WindowEvent::HoveredFileCancelled => {
                            let mut events =
                                world.get_resource_mut::<Events<FileDragAndDrop>>().unwrap();
                            events.send(FileDragAndDrop::HoveredFileCancelled { id: window_id });
                        }
                        WindowEvent::Moved(position) => {
                            let position = ivec2(position.x, position.y);
                            window.update_actual_position_from_backend(position);
                            let mut events =
                                world.get_resource_mut::<Events<WindowMoved>>().unwrap();
                            events.send(WindowMoved {
                                id: window_id,
                                position,
                            });
                        }
                        _ => {}
                    }
                }
                Event::UserEvent(user_event) => match user_event {
                    UserEvent::WindowEvent(user_window_event) => {
                        let world = app.world.cell();
                        let mut windows = world.get_non_send_mut::<Windows>().unwrap();
                        let window = windows.get_primary_mut().unwrap();
                        let id = WindowId::primary();

                        let dioxus_windows = world.get_non_send::<DioxusWindows>().unwrap();
                        let dioxus_window = dioxus_windows.get(id).unwrap();
                        let tao_window = dioxus_windows.get_tao_window(id).unwrap();

                        match user_window_event {
                            UserWindowEvent::Update => {
                                let mut events =
                                    world.get_resource_mut::<Events<DomUpdated>>().unwrap();

                                dioxus_window.try_load_ready_webview();
                                events.send(DomUpdated { id });
                            }
                            UserWindowEvent::CloseWindow => {
                                let mut events = world
                                    .get_resource_mut::<Events<WindowCloseRequested>>()
                                    .unwrap();
                                events.send(WindowCloseRequested { id });
                            }
                            UserWindowEvent::DragWindow => {
                                let mut events =
                                    world.get_resource_mut::<Events<WindowDragged>>().unwrap();

                                if tao_window.fullscreen().is_none() {
                                    if let Ok(()) = tao_window.drag_window() {
                                        events.send(WindowDragged { id });
                                    }
                                }
                                events.send(WindowDragged { id });
                            }
                            UserWindowEvent::Visible(visible) => {
                                let mut events =
                                    world.get_resource_mut::<Events<VisibleUpdated>>().unwrap();

                                tao_window.set_visible(visible);
                                events.send(VisibleUpdated { id, visible });
                            }
                            UserWindowEvent::Minimize(minimized) => {
                                let mut events =
                                    world.get_resource_mut::<Events<WindowMinimized>>().unwrap();

                                window.set_minimized(minimized);
                                events.send(WindowMinimized { id, minimized });
                            }
                            UserWindowEvent::Maximize(maximized) => {
                                let mut events =
                                    world.get_resource_mut::<Events<WindowMaximized>>().unwrap();

                                window.set_maximized(maximized);
                                events.send(WindowMaximized { id, maximized });
                            }
                            UserWindowEvent::MaximizeToggle => {
                                tao_window.set_maximized(!tao_window.is_maximized())
                            }
                            // Fullscreen(state) => {
                            //     if let Some(handle) = tao_window.current_monitor() {
                            //         tao_window.set_fullscreen(
                            //             state.then(|| WryFullscreen::Borderless(Some(handle))),
                            //         );
                            //     }
                            // }
                            // FocusWindow => tao_window.set_focus(),
                            // Resizable(state) => tao_window.set_resizable(state),
                            // AlwaysOnTop(state) => tao_window.set_always_on_top(state),
                            // CursorVisible(state) => tao_window.set_cursor_visible(state),
                            // CursorGrab(state) => {
                            //     let _ = tao_window.set_cursor_grab(state);
                            // }
                            // SetTitle(content) => tao_window.set_title(&content),
                            // SetDecorations(state) => tao_window.set_decorations(state),
                            // SetZoomLevel(scale_factor) => webview.zoom(scale_factor),
                            // Print => {
                            //     if let Err(e) = webview.print() {
                            //         // we can't panic this error.
                            //         log::warn!("Open print modal failed: {e}");
                            //     }
                            // }
                            // DevTool => webview.open_devtools(),
                            // Eval(code) => webview
                            //     .evaluate_script(code.as_str())
                            //     .expect("eval shouldn't panic"),
                            _ => {}
                        };
                    }
                    UserEvent::CoreCommand(cmd) => {
                        let mut events = app
                            .world
                            .get_resource_mut::<Events<CoreCommand>>()
                            .expect("Provide CoreCommand event to bevy");
                        events.send(cmd);
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

struct TaoPersistentState {
    active: bool,
    low_power_event: bool,
    redraw_request_sent: bool,
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
