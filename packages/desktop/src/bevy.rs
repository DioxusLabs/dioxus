use crate::{
    cfg::DesktopConfig,
    controller::DesktopController,
    desktop_context::{self, UserWindowEvent},
    events::{parse_ipc_message, trigger_from_serialized},
    protocol,
    tao::{
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::Window,
    },
};
use bevy::prelude::*;
use dioxus_core::Component;
use dioxus_core::*;
use futures_channel::mpsc;
use futures_util::stream::StreamExt;
use std::{fmt::Debug, marker::PhantomData};
use tokio::sync::broadcast::{channel, Sender};
pub use wry;
pub use wry::application as tao;
use wry::webview::WebViewBuilder;

pub struct DioxusDesktopPlugin<Props, CoreCommand, UICommand> {
    root: Component<Props>,
    props: Props,
    core_cmd_type: PhantomData<CoreCommand>,
    ui_cmd_type: PhantomData<UICommand>,
}

impl<
        Props: 'static + Send + Sync + Copy,
        CoreCommand: 'static + Send + Sync + Debug + Clone,
        UICommand: 'static + Send + Sync + Clone + Copy,
    > Plugin for DioxusDesktopPlugin<Props, CoreCommand, UICommand>
{
    fn build(&self, app: &mut App) {
        app.add_event::<CoreCommand>()
            .add_event::<UICommand>()
            .insert_resource(DioxusDesktop::<Props, CoreCommand, UICommand>::new(
                self.root, self.props,
            ))
            .set_runner(|app| DioxusDesktop::<Props, CoreCommand, UICommand>::runner(app))
            .add_system_to_stage(
                CoreStage::Last,
                dispatch_ui_commands::<Props, CoreCommand, UICommand>,
            );
    }
}

impl<Props, CoreCommand, UICommand> DioxusDesktopPlugin<Props, CoreCommand, UICommand> {
    pub fn new(root: Component<Props>, props: Props) -> Self {
        Self {
            root,
            props,
            core_cmd_type: PhantomData,
            ui_cmd_type: PhantomData,
        }
    }
}

pub struct DioxusDesktop<Props, CoreCommand, UICommand> {
    root: Component<Props>,
    props: Props,
    sender: Option<Sender<UICommand>>,
    data: PhantomData<CoreCommand>,
}

impl<Props, CoreCommand, UICommand> DioxusDesktop<Props, CoreCommand, UICommand> {
    pub fn new(root: Component<Props>, props: Props) -> Self {
        Self {
            root,
            props,
            sender: None,
            data: PhantomData,
        }
    }

    fn sender(&self) -> Sender<UICommand> {
        self.sender
            .clone()
            .expect("Sender<UICommand> isn't initialized")
    }

    fn set_sender(&mut self, sender: Sender<UICommand>) {
        self.sender = Some(sender);
    }
}

impl<
        Props: 'static + Send + Sync + Copy,
        CoreCommand: 'static + Send + Sync + Debug + Clone,
        UICommand: 'static + Send + Sync + Clone,
    > DioxusDesktop<Props, CoreCommand, UICommand>
{
    fn runner(mut app: App) {
        let mut cfg = DesktopConfig::default().with_default_icon();
        // builder(&mut cfg);
        let event_loop = EventLoop::<UserWindowEvent<CoreCommand>>::with_user_event();

        let (core_tx, mut core_rx) = mpsc::unbounded::<CoreCommand>();
        let (ui_tx, _) = channel::<UICommand>(8);

        let mut desktop_resource = app
            .world
            .get_resource_mut::<DioxusDesktop<Props, CoreCommand, UICommand>>()
            .expect("Provide DioxusDesktopConfig resource");

        desktop_resource.set_sender(ui_tx.clone());

        let mut desktop = DesktopController::new_on_tokio::<Props, CoreCommand, UICommand>(
            desktop_resource.root,
            desktop_resource.props,
            event_loop.create_proxy(),
            Some((core_tx, ui_tx)),
        );
        let proxy = event_loop.create_proxy();

        let proxy_clone = proxy.clone();
        let runtime = tokio::runtime::Runtime::new().expect("Failed to initialize runtime");
        runtime.spawn(async move {
            while let Some(cmd) = core_rx.next().await {
                let _res = proxy_clone.send_event(UserWindowEvent::BevyUpdate(cmd));
            }
        });

        app.update();

        event_loop.run(move |window_event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;

            match window_event {
                Event::NewEvents(StartCause::Init) => {
                    let builder = cfg.window.clone();

                    let window = builder.build(event_loop).unwrap();
                    let window_id = window.id();

                    let (is_ready, sender) = (desktop.is_ready.clone(), desktop.sender.clone());

                    let proxy = proxy.clone();
                    let file_handler = cfg.file_drop_handler.take();

                    let resource_dir = cfg.resource_dir.clone();

                    let mut webview = WebViewBuilder::new(window)
                        .unwrap()
                        .with_transparent(cfg.window.window.transparent)
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

                    for (name, handler) in cfg.protocols.drain(..) {
                        webview = webview.with_custom_protocol(name, handler)
                    }

                    if cfg.disable_context_menu {
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
        })
    }
}

fn dispatch_ui_commands<
    Props: 'static + Send + Sync,
    CoreCommand: 'static + Send + Sync,
    UICommand: 'static + Send + Sync + Copy,
>(
    mut events: EventReader<UICommand>,
    desktop: Res<DioxusDesktop<Props, CoreCommand, UICommand>>,
) {
    let tx = desktop.sender();
    for e in events.iter() {
        let _ = tx.send(*e);
    }
}
