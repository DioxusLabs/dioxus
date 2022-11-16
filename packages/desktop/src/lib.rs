#![doc = include_str!("readme.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]

mod cfg;
mod controller;
mod desktop_context;
mod escape;
mod events;
#[cfg(any(feature = "hot-reload", debug_assertions))]
mod hot_reload;
mod protocol;

use desktop_context::UserWindowEvent;
pub use desktop_context::{use_eval, use_window, DesktopContext};
pub use wry;
pub use wry::application as tao;

pub use cfg::Config;
use controller::DesktopController;
use dioxus_core::*;
use events::parse_ipc_message;
pub use tao::dpi::{LogicalSize, PhysicalSize};
pub use tao::window::WindowBuilder;
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use wry::webview::WebViewBuilder;

/// Launch the WebView and run the event loop.
///
/// This function will start a multithreaded Tokio runtime as well the WebView event loop.
///
/// ```rust, ignore
/// use dioxus::prelude::*;
///
/// fn main() {
///     dioxus_desktop::launch(app);
/// }
///
/// fn app(cx: Scope) -> Element {
///     cx.render(rsx!{
///         h1 {"hello world!"}
///     })
/// }
/// ```
pub fn launch(root: Component) {
    launch_with_props(root, (), Config::default())
}

/// Launch the WebView and run the event loop, with configuration.
///
/// This function will start a multithreaded Tokio runtime as well the WebView event loop.
///
/// You can configure the WebView window with a configuration closure
///
/// ```rust, ignore
/// use dioxus::prelude::*;
///
/// fn main() {
///     dioxus_desktop::launch_cfg(app, |c| c.with_window(|w| w.with_title("My App")));
/// }
///
/// fn app(cx: Scope) -> Element {
///     cx.render(rsx!{
///         h1 {"hello world!"}
///     })
/// }
/// ```
pub fn launch_cfg(root: Component, config_builder: Config) {
    launch_with_props(root, (), config_builder)
}

/// Launch the WebView and run the event loop, with configuration and root props.
///
/// This function will start a multithreaded Tokio runtime as well the WebView event loop.
///
/// You can configure the WebView window with a configuration closure
///
/// ```rust, ignore
/// use dioxus::prelude::*;
///
/// fn main() {
///     dioxus_desktop::launch_cfg(app, AppProps { name: "asd" }, |c| c);
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
pub fn launch_with_props<P: 'static + Send>(root: Component<P>, props: P, mut cfg: Config) {
    let event_loop = EventLoop::with_user_event();

    let mut desktop = DesktopController::new_on_tokio(root, props, event_loop.create_proxy());
    let proxy = event_loop.create_proxy();

    // We assume that if the icon is None, then the user just didnt set it
    if cfg.window.window.window_icon.is_none() {
        cfg.window = cfg.window.with_window_icon(Some(
            tao::window::Icon::from_rgba(
                include_bytes!("./assets/default_icon.bin").to_vec(),
                460,
                460,
            )
            .expect("image parse failed"),
        ));
    }

    event_loop.run(move |window_event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        // println!("window event: {:?}", window_event);
        match window_event {
            Event::NewEvents(StartCause::Init) => {
                let builder = cfg.window.clone();

                let window = builder.build(event_loop).unwrap();
                let window_id = window.id();

                let (is_ready, _) = (desktop.is_ready.clone(), ());
                // let (is_ready, sender) = (desktop.is_ready.clone(), desktop.sender.clone());

                let proxy = proxy.clone();

                let file_handler = cfg.file_drop_handler.take();
                let custom_head = cfg.custom_head.clone();
                let resource_dir = cfg.resource_dir.clone();
                let index_file = cfg.custom_index.clone();

                let mut webview = WebViewBuilder::new(window)
                    .unwrap()
                    .with_transparent(cfg.window.window.transparent)
                    .with_url("dioxus://index.html/")
                    .unwrap()
                    .with_ipc_handler(move |_window: &Window, payload: String| {
                        parse_ipc_message(&payload)
                            .map(|message| match message.method() {
                                "user_event" => {
                                    println!("user event!");
                                    // let event = trigger_from_serialized(message.params());
                                    // log::trace!("User event: {:?}", event);
                                    // sender.unbounded_send(SchedulerMsg::Event(event)).unwrap();
                                }
                                "initialize" => {
                                    is_ready.store(true, std::sync::atomic::Ordering::Relaxed);
                                    println!("initializing...");
                                    let _ = proxy.send_event(UserWindowEvent::Update);
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
                    .with_custom_protocol(String::from("dioxus"), move |r| {
                        protocol::desktop_handler(
                            r,
                            resource_dir.clone(),
                            custom_head.clone(),
                            index_file.clone(),
                        )
                    })
                    // passing edits via the custom protocol is faster than using eval, maybe?
                    .with_custom_protocol(String::from("edits"), move |r| {
                        //
                        // Ok(Response::body())
                        todo!()
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
                            e.preventDefault();
                        }, false);
                        } else {
                        document.attachEvent('oncontextmenu', function() {
                            window.event.returnValue = false;
                        });
                        }
                    "#,
                    )
                } else {
                    // in debug, we are okay with the reload menu showing and dev tool
                    webview = webview.with_devtools(true);
                }

                desktop.webviews.insert(window_id, webview.build().unwrap());
            }

            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Destroyed { .. } => desktop.close_window(window_id, control_flow),
                _ => {}
            },

            Event::UserEvent(user_event) => {
                desktop_context::handler(user_event, &mut desktop, control_flow)
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
