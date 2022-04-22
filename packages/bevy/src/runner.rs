use crate::event::{parse_keyboard_event, CustomUserEvent};
use bevy::{
    app::{App, AppExit},
    ecs::event::{Events, ManualEventReader},
    input::keyboard::KeyboardInput,
};
use dioxus_core::SchedulerMsg;
use dioxus_desktop::{
    cfg::DesktopConfig,
    controller::DesktopController,
    desktop_context::{user_window_event_handler, UserEvent, UserWindowEvent},
    events::{parse_ipc_message, trigger_from_serialized},
    protocol,
    tao::{
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
        window::Window,
    },
};
use futures_intrusive::channel::shared::Receiver;
use std::fmt::Debug;
use tokio::runtime::Runtime;
use wry::webview::WebViewBuilder;

pub fn runner<CoreCommand, UICommand>(mut app: App)
where
    CoreCommand: 'static + Send + Sync + Debug,
    UICommand: 'static,
{
    let event_loop = app
        .world
        .remove_non_send_resource::<EventLoop<UserEvent<CustomUserEvent<CoreCommand>>>>()
        .expect("Insert EventLoop as non send resource");
    let mut desktop = app
        .world
        .remove_non_send_resource::<DesktopController>()
        .expect("Insert DesktopController as non send resource");
    let mut config = app
        .world
        .remove_non_send_resource::<DesktopConfig>()
        .unwrap_or_default();
    let core_rx = app
        .world
        .remove_resource::<Receiver<CoreCommand>>()
        .expect("Failed to retrieve CoreCommand receiver resource");

    let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
    app.world
        .insert_non_send_resource(event_loop.create_proxy());

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

    app.update();

    event_loop.run(
        move |window_event: Event<UserEvent<CustomUserEvent<CoreCommand>>>,
              event_loop: &EventLoopWindowTarget<UserEvent<CustomUserEvent<CoreCommand>>>,
              control_flow: &mut ControlFlow| {
            *control_flow = ControlFlow::Wait;

            if let Some(app_exit_events) = app.world.get_resource_mut::<Events<AppExit>>() {
                if app_exit_event_reader
                    .iter(&app_exit_events)
                    .next_back()
                    .is_some()
                {
                    *control_flow = ControlFlow::Exit;
                }
            }

            match window_event {
                Event::NewEvents(StartCause::Init) => {
                    let builder = config.window.clone();

                    let window = builder.build(event_loop).unwrap();
                    let window_id = window.id();

                    let (is_ready, sender) = (desktop.is_ready.clone(), desktop.sender.clone());

                    let file_handler = config.file_drop_handler.take();

                    let resource_dir = config.resource_dir.clone();
                    let world = app.world.cell();
                    let proxy = world
                        .get_non_send_mut::<EventLoopProxy<UserEvent<CustomUserEvent<CoreCommand>>>>()
                        .unwrap()
                        .clone();

                    let mut webview = WebViewBuilder::new(window)
                        .unwrap()
                        .with_transparent(config.window.window.transparent)
                        .with_url("dioxus://index.html/")
                        .unwrap()
                        .with_ipc_handler(move |_window: &Window, payload: String| {
                            parse_ipc_message(&payload)
                                .map(|message| match message.method() {
                                    "user_event" => {
                                        let event = trigger_from_serialized(message.params());
                                        sender.unbounded_send(SchedulerMsg::Event(event)).unwrap();
                                    }
                                    "keyboard_event" => {
                                        let input = parse_keyboard_event(message.params());
                                        proxy.send_event(UserEvent::CustomEvent(CustomUserEvent::KeyboardInput(input))).unwrap();
                                    }
                                    "initialize" => {
                                        is_ready.store(true, std::sync::atomic::Ordering::Relaxed);
                                        let _ = proxy.send_event(UserEvent::WindowEvent(
                                            UserWindowEvent::Update,
                                        ));
                                    }
                                    "browser_open" => {
                                        let data = message.params();
                                        log::trace!("Open browser: {:?}", data);
                                        if let Some(temp) = data.as_object() {
                                            if temp.contains_key("href") {
                                                let url =
                                                    temp.get("href").unwrap().as_str().unwrap();
                                                if let Err(e) = webbrowser::open(url) {
                                                    log::error!("Open Browser error: {:?}", e);
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                })
                                .unwrap_or_else(|| {
                                    log::warn!("invalid IPC message received");
                                })
                        })
                        .with_custom_protocol(String::from("dioxus"), move |r| {
                            protocol::desktop_handler(r, resource_dir.clone())
                        })
                        .with_file_drop_handler(move |window, evet| {
                            file_handler
                                .as_ref()
                                .map(|handler| handler(window, evet))
                                .unwrap_or_default()
                        });

                    for (name, handler) in config.protocols.drain(..) {
                        webview = webview.with_custom_protocol(name, handler)
                    }

                    if config.disable_context_menu {
                        // in release mode, we don't want to show the dev tool or reload menus
                        webview = webview.with_initialization_script(
                            r#"
                        if (document.addEventListener) {
                            document.addEventListener('contextmenu', function(e) {
                                alert("You've tried to open context menu");
                                e.preventDefault();
                            }, false);
                        } else {
                            document.attachEvent('oncontextmenu', function() {
                                alert("You've tried to open context menu");
                                window.event.returnValue = false;
                            });
                        }
                    "#,
                        )
                    } else {
                        // in debug, we are okay with the reload menu showing and dev tool
                        webview = webview.with_dev_tool(true);
                    }

                    desktop.webviews.insert(window_id, webview.build().unwrap());
                }

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
