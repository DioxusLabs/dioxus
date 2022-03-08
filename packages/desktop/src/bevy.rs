use crate::{
    cfg::DesktopConfig,
    controller::DesktopController,
    desktop_context::{self, UserWindowEvent},
    events::{parse_ipc_message, trigger_from_serialized},
    protocol,
    tao::{
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
        window::Window,
    },
};
use bevy::{
    app::{App, AppExit, CoreStage, Plugin},
    ecs::{
        event::{EventReader, Events, ManualEventReader},
        system::Res,
    },
};
use dioxus_core::Component;
use dioxus_core::*;
use futures_channel::mpsc::{unbounded, UnboundedReceiver};
use futures_util::stream::StreamExt;
use std::{fmt::Debug, marker::PhantomData};
use tokio::{
    runtime::Runtime,
    sync::broadcast::{channel, Sender},
};
pub use wry;
pub use wry::application as tao;
use wry::webview::WebViewBuilder;

pub struct DioxusDesktopPlugin<CoreCommand, UICommand, Props = ()> {
    root: Component<Props>,
    props: Props,
    core_cmd_type: PhantomData<CoreCommand>,
    ui_cmd_type: PhantomData<UICommand>,
}

impl<
        CoreCommand: 'static + Send + Sync + Clone + Debug,
        UICommand: 'static + Send + Sync + Clone + Copy,
        Props: 'static + Send + Sync + Copy,
    > Plugin for DioxusDesktopPlugin<CoreCommand, UICommand, Props>
{
    fn build(&self, app: &mut App) {
        let config = DesktopConfig::default().with_default_icon();
        // builder(&mut config);
        let event_loop = EventLoop::<UserWindowEvent<CoreCommand>>::with_user_event();

        let (core_tx, core_rx) = unbounded::<CoreCommand>();
        let (ui_tx, _) = channel::<UICommand>(8);

        let desktop = DesktopController::new_on_tokio::<CoreCommand, UICommand, Props>(
            self.root,
            self.props,
            event_loop.create_proxy(),
            Some((core_tx, ui_tx.clone())),
        );

        app.add_event::<CoreCommand>()
            .add_event::<UICommand>()
            .insert_non_send_resource(event_loop)
            .insert_non_send_resource(desktop)
            .insert_non_send_resource(config)
            .insert_resource(ui_tx)
            .insert_resource(core_rx)
            .set_runner(|app| runner::<CoreCommand, UICommand>(app))
            .add_system_to_stage(CoreStage::Last, dispatch_ui_commands::<UICommand>);
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

fn runner<CoreCommand: 'static + Send + Sync + Debug, UICommand: 'static>(mut app: App) {
    let event_loop = app
        .world
        .remove_non_send_resource::<EventLoop<UserWindowEvent<CoreCommand>>>()
        .unwrap();
    let mut desktop = app
        .world
        .remove_non_send_resource::<DesktopController>()
        .unwrap();
    let mut config = app
        .world
        .remove_non_send_resource::<DesktopConfig>()
        .unwrap();
    let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
    app.world
        .insert_non_send_resource(event_loop.create_proxy());

    let runtime = Runtime::new().unwrap();
    let mut core_rx = app
        .world
        .remove_resource::<UnboundedReceiver<CoreCommand>>()
        .unwrap();
    let proxy = event_loop.create_proxy();

    runtime.spawn(async move {
        while let Some(cmd) = core_rx.next().await {
            let _res = proxy.clone().send_event(UserWindowEvent::BevyUpdate(cmd));
        }
    });

    app.update();

    event_loop.run(
        move |window_event: Event<UserWindowEvent<CoreCommand>>,
              event_loop: &EventLoopWindowTarget<UserWindowEvent<CoreCommand>>,
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
                        .get_non_send_mut::<EventLoopProxy<UserWindowEvent<CoreCommand>>>()
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
                                    "initialize" => {
                                        is_ready.store(true, std::sync::atomic::Ordering::Relaxed);
                                        let _ = proxy.send_event(UserWindowEvent::Update);
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
                                    _ => (),
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

                Event::UserEvent(user_event) => {
                    desktop_context::handler(user_event, &mut desktop, control_flow, Some(&mut app))
                }
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

fn dispatch_ui_commands<UICommand: 'static + Send + Sync + Copy>(
    mut events: EventReader<UICommand>,
    tx: Res<Sender<UICommand>>,
) {
    for e in events.iter() {
        let _ = tx.send(*e);
    }
}
