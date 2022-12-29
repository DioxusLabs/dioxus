#![doc = include_str!("readme.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]

mod cfg;
mod controller;
mod desktop_context;
mod escape;
mod events;
mod protocol;

#[cfg(all(feature = "hot-reload", debug_assertions))]
mod hot_reload;

use futures_util::task::ArcWake;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::task::Waker;

use desktop_context::UserWindowEvent;
pub use desktop_context::{use_eval, use_window, DesktopContext, EvalResult};
use futures_channel::mpsc::UnboundedSender;
use futures_util::future::poll_fn;
use futures_util::{pin_mut, FutureExt};
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
use wry::application::event_loop::EventLoopProxy;
use wry::application::platform::run_return::EventLoopExtRunReturn;
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
pub async fn launch(root: Component) {
    launch_with_props(root, (), Config::default()).await
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
pub async fn launch_cfg(root: Component, config_builder: Config) {
    launch_with_props(root, (), config_builder).await
}

/// Launch the WebView and run the event loop, with configuration and root props.
///
/// THIS WILL BLOCK THE CURRENT THREAD
///
/// This function will start a multithreaded Tokio runtime as well the WebView event loop.
///
/// You can configure the WebView window with a configuration closure
///
/// ```rust, ignore
/// use dioxus::prelude::*;
///
/// fn main() {
///     dioxus_desktop::launch_with_props(app, AppProps { name: "asd" }, |c| c);
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
pub async fn launch_with_props<P: 'static + Send>(root: Component<P>, props: P, mut cfg: Config) {
    let mut event_loop = EventLoop::with_user_event();

    let is_ready = Arc::new(AtomicBool::new(false));
    let (eval_sender, eval_reciever) = tokio::sync::mpsc::unbounded_channel();

    let mut dom = VirtualDom::new_with_props(root, props).with_root_context(DesktopContext::new(
        event_loop.create_proxy(),
        eval_reciever,
    ));

    let proxy = event_loop.create_proxy();

    let waker = futures_util::task::waker(Arc::new(DomHandle {
        proxy: proxy.clone(),
    }));

    let mut events = Rc::new(RefCell::new(vec![]));
    let mut webviews = HashMap::new();

    event_loop.run(move |window_event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match window_event {
            Event::NewEvents(StartCause::Init) => {
                let window = build_webview(
                    &mut cfg,
                    event_loop,
                    is_ready.clone(),
                    proxy.clone(),
                    eval_sender.clone(),
                    events.clone(),
                );

                webviews.insert(window.window().id(), window);

                // desktop.start(&mut cfg, event_loop);
            }

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Destroyed { .. } => {
                    // desktop.close_window(window_id, control_flow);
                }
                _ => {}
            },

            Event::UserEvent(user_event) => {
                println!("user event: {:?}", user_event);

                match user_event {
                    UserWindowEvent::Poll => {
                        let mut cx = std::task::Context::from_waker(&waker);

                        let render = {
                            let fut = dom.wait_for_work();
                            pin_mut!(fut);
                            matches!(fut.poll_unpin(&mut cx), std::task::Poll::Ready(_))
                        };

                        if render {
                            let edits = dom.render_immediate();

                            // apply the edits
                        }
                    }

                    UserWindowEvent::EditsReady => {
                        let edits = dom.rebuild();

                        let (_id, view) = webviews.iter_mut().next().unwrap();

                        let serialized = serde_json::to_string(&edits).unwrap();

                        view.evaluate_script(&format!(
                            "window.interpreter.handleEdits({})",
                            serialized
                        ))
                        .unwrap();
                    }

                    other => {
                        // desktop.handle_event(user_event, control_flow);
                    }
                }
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

struct DomHandle {
    proxy: EventLoopProxy<UserWindowEvent>,
}

impl ArcWake for DomHandle {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.proxy.send_event(UserWindowEvent::Poll).unwrap();
    }
}

fn build_webview(
    cfg: &mut Config,
    event_loop: &tao::event_loop::EventLoopWindowTarget<UserWindowEvent>,
    is_ready: Arc<AtomicBool>,
    proxy: tao::event_loop::EventLoopProxy<UserWindowEvent>,
    eval_sender: tokio::sync::mpsc::UnboundedSender<serde_json::Value>,
    event_tx: Rc<RefCell<Vec<Value>>>,
) -> wry::webview::WebView {
    let builder = cfg.window.clone();
    let window = builder.build(event_loop).unwrap();
    let file_handler = cfg.file_drop_handler.take();
    let custom_head = cfg.custom_head.clone();
    let resource_dir = cfg.resource_dir.clone();
    let index_file = cfg.custom_index.clone();
    let root_name = cfg.root_name.clone();

    // We assume that if the icon is None in cfg, then the user just didnt set it
    if cfg.window.window.window_icon.is_none() {
        window.set_window_icon(Some(
            tao::window::Icon::from_rgba(
                include_bytes!("./assets/default_icon.bin").to_vec(),
                460,
                460,
            )
            .expect("image parse failed"),
        ));
    }

    let mut webview = WebViewBuilder::new(window)
        .unwrap()
        .with_transparent(cfg.window.window.transparent)
        .with_url("dioxus://index.html/")
        .unwrap()
        .with_ipc_handler(move |_window: &Window, payload: String| {
            let message = match parse_ipc_message(&payload) {
                Some(message) => message,
                None => {
                    log::error!("Failed to parse IPC message: {}", payload);
                    return;
                }
            };

            match message.method() {
                "eval_result" => {
                    let result = message.params();
                    eval_sender.send(result).unwrap();
                }
                "user_event" => {
                    _ = event_tx.borrow_mut().push(message.params());
                    let _ = proxy.send_event(UserWindowEvent::Poll);
                }
                "initialize" => {
                    is_ready.store(true, std::sync::atomic::Ordering::Relaxed);
                    let _ = proxy.send_event(UserWindowEvent::EditsReady);
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
            }
        })
        .with_custom_protocol(String::from("dioxus"), move |r| {
            protocol::desktop_handler(
                r,
                resource_dir.clone(),
                custom_head.clone(),
                index_file.clone(),
                &root_name,
            )
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

    webview.build().unwrap()
}
