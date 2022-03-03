#![doc = include_str!("readme.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub mod cfg;
mod controller;
pub mod desktop_context;
pub mod escape;
pub mod events;
mod protocol;
mod user_window_events;

use cfg::DesktopConfig;
use controller::DesktopController;
pub use desktop_context::use_window;
use dioxus_core::Component;
use dioxus_core::*;
use events::parse_ipc_message;
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
pub use wry;
pub use wry::application as tao;
use wry::webview::WebViewBuilder;

pub use bevy::prelude::{Component as BevyComponent, *};
pub use std::{fmt::Debug, marker::PhantomData};
pub use tokio::sync::{
    broadcast::{channel, Receiver, Sender},
    mpsc,
};

use crate::events::trigger_from_serialized;

/// Launch the WebView and run the event loop.
///
/// This function will start a multithreaded Tokio runtime as well the WebView event loop.
///
/// ```rust
/// use dioxus::prelude::*;
///
/// fn main() {
///     dioxus::desktop::launch(app);
/// }
///
/// fn app(cx: Scope) -> Element {
///     cx.render(rsx!{
///         h1 {"hello world!"}
///     })
/// }
/// ```
pub fn launch(root: Component) {
    launch_with_props(root, (), |c| c)
}

/// Launch the WebView and run the event loop, with configuration.
///
/// This function will start a multithreaded Tokio runtime as well the WebView event loop.
///
/// You can configure the WebView window with a configuration closure
///
/// ```rust
/// use dioxus::prelude::*;
///
/// fn main() {
///     dioxus::desktop::launch_cfg(app, |c| c.with_window(|w| w.with_title("My App")));
/// }
///
/// fn app(cx: Scope) -> Element {
///     cx.render(rsx!{
///         h1 {"hello world!"}
///     })
/// }
/// ```
pub fn launch_cfg(
    root: Component,
    config_builder: impl FnOnce(&mut DesktopConfig) -> &mut DesktopConfig,
) {
    launch_with_props(root, (), config_builder)
}

/// Launch the WebView and run the event loop, with configuration and root props.
///
/// This function will start a multithreaded Tokio runtime as well the WebView event loop.
///
/// You can configure the WebView window with a configuration closure
///
/// ```rust
/// use dioxus::prelude::*;
///
/// fn main() {
///     dioxus::desktop::launch_cfg(app, AppProps { name: "asd" }, |c| c);
/// }
///
/// struct AppProps {
///     name: &'static str
/// }
///
/// fn app(cx: Scope<AppProps>) -> Element {
///     cx.render(rsx!{
///         h1 {"hello {cx.props.name}!"}
///     })
/// }
/// ```
pub fn launch_with_props<P: 'static + Send>(
    root: Component<P>,
    props: P,
    builder: impl FnOnce(&mut DesktopConfig) -> &mut DesktopConfig,
) {
    let mut cfg = DesktopConfig::default().with_default_icon();
    builder(&mut cfg);

    let event_loop = EventLoop::with_user_event();

    let mut desktop = DesktopController::new_on_tokio(root, props, event_loop.create_proxy());
    let proxy = event_loop.create_proxy();

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
                                    log::trace!("User event: {:?}", event);
                                    sender.unbounded_send(SchedulerMsg::Event(event)).unwrap();
                                }
                                "initialize" => {
                                    is_ready.store(true, std::sync::atomic::Ordering::Relaxed);
                                    let _ = proxy
                                        .send_event(user_window_events::UserWindowEvent::Update);
                                }
                                "browser_open" => {
                                    let data = message.params();
                                    log::trace!("Open browser: {:?}", data);
                                    if let Some(temp) = data.as_object() {
                                        if temp.contains_key("href") {
                                            let url = temp.get("href").unwrap().as_str().unwrap();
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
                            });
                    })
                    .with_custom_protocol(String::from("dioxus"), protocol::desktop_handler)
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
                user_window_events::handler(user_event, &mut desktop, control_flow)
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

pub struct DioxusDesktopPlugin<CoreCommand, UICommand> {
    pub root: Component<AppProps<CoreCommand, UICommand>>,
    pub core_cmd_type: PhantomData<CoreCommand>,
    pub ui_cmd_type: PhantomData<UICommand>,
}

impl<CoreCommand: 'static + Send + Sync + Debug, UICommand: 'static + Send + Sync + Clone> Plugin
    for DioxusDesktopPlugin<CoreCommand, UICommand>
{
    fn build(&self, app: &mut App) {
        app.insert_resource(DioxusDesktop {
            root: self.root,
            sender: None,
        })
        .set_runner(|app| DioxusDesktop::<CoreCommand, UICommand>::runner(app));
    }
}

pub struct DioxusDesktop<CoreCommand, UICommand> {
    root: Component<AppProps<CoreCommand, UICommand>>,
    sender: Option<Sender<UICommand>>,
}

impl<CoreCommand, UICommand> DioxusDesktop<CoreCommand, UICommand> {
    pub fn sender(&self) -> Sender<UICommand> {
        self.sender
            .clone()
            .expect("Sender<UICommand> isn't initialized")
    }
}

impl<CoreCommand, UICommand> DioxusDesktop<CoreCommand, UICommand> {
    fn set_sender(&mut self, sender: Sender<UICommand>) {
        self.sender = Some(sender);
    }
}

pub struct AppProps<CoreCommand, UICommand> {
    pub channel: (mpsc::UnboundedSender<CoreCommand>, Sender<UICommand>),
}

impl<CoreCommand: 'static + Send + Debug, UICommand: 'static + Send + Clone>
    DioxusDesktop<CoreCommand, UICommand>
{
    fn runner(mut app: App) {
        let mut cfg = DesktopConfig::default().with_default_icon();
        // builder(&mut cfg);
        let event_loop = EventLoop::with_user_event();

        let (core_tx, mut core_rx) = mpsc::unbounded_channel::<CoreCommand>();
        let (ui_tx, _) = channel::<UICommand>(8);

        let mut desktop_resource = app
            .world
            .get_resource_mut::<DioxusDesktop<CoreCommand, UICommand>>()
            .expect("Provide DioxusDesktopConfig resource");

        desktop_resource.set_sender(ui_tx.clone());

        let props = AppProps::<CoreCommand, UICommand> {
            channel: (core_tx, ui_tx),
        };

        let mut desktop = DesktopController::new_on_tokio::<AppProps<CoreCommand, UICommand>>(
            desktop_resource.root,
            props,
            event_loop.create_proxy(),
        );
        let proxy = event_loop.create_proxy();

        let runtime = tokio::runtime::Runtime::new().expect("Failed to initialize runtime");

        runtime.spawn(async move {
            while let Some(cmd) = core_rx.recv().await {
                println!("🧠 {:?}", cmd);
            }
        });

        app.run();

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
                                        let _ = proxy.send_event(
                                            user_window_events::UserWindowEvent::Update,
                                        );
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
                        .with_custom_protocol(String::from("dioxus"), protocol::desktop_handler)
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
                    user_window_events::handler(user_event, &mut desktop, control_flow)
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
