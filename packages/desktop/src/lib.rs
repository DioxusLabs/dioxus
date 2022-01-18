//! Dioxus Desktop Renderer
//!
//! Render the Dioxus VirtualDom using the platform's native WebView implementation.
//!
//! # Desktop
//!
//! One of Dioxus' killer features is the ability to quickly build a native desktop app that looks and feels the same across platforms. Apps built with Dioxus are typically <5mb in size and use existing system resources, so they won't hog extreme amounts of RAM or memory.
//!
//! Dioxus Desktop is built off Tauri. Right now there aren't any Dioxus abstractions over keyboard shortcuts, menubar, handling, etc, so you'll want to leverage Tauri - mostly [Wry](http://github.com/tauri-apps/wry/) and [Tao](http://github.com/tauri-apps/tao)) directly. The next major release of Dioxus-Desktop will include components and hooks for notifications, global shortcuts, menubar, etc.
//!
//!
//! ## Getting Set up
//!
//! Getting Set up with Dioxus-Desktop is quite easy. Make sure you have Rust and Cargo installed, and then create a new project:
//!
//! ```shell
//! $ cargo new --bin demo
//! $ cd app
//! ```
//!
//! Add Dioxus with the `desktop` feature:
//!
//! ```shell
//! $ cargo add dioxus --features desktop
//! ```
//!
//! Edit your `main.rs`:
//!
//! ```rust
//! // main.rs
//! use dioxus::prelude::*;
//!
//! fn main() {
//!     dioxus::desktop::launch(app);
//! }
//!
//! fn app(cx: Scope) -> Element {
//!     cx.render(rsx!{
//!         div {
//!             "hello world!"
//!         }
//!     })
//! }
//! ```
//!
//!
//! To configure the webview, menubar, and other important desktop-specific features, checkout out some of the launch configuration in the [API reference](https://docs.rs/dioxus-desktop/).
//!
//! ## Future Steps
//!
//! Make sure to read the [Dioxus Guide](https://dioxuslabs.com/guide) if you already haven't!

pub mod cfg;
pub mod escape;
pub mod events;

use cfg::DesktopConfig;
use dioxus_core::*;
use std::{
    collections::{HashMap, VecDeque},
    sync::atomic::AtomicBool,
    sync::{Arc, RwLock},
};
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowId},
};
pub use wry;
pub use wry::application as tao;
use wry::{
    application::event_loop::EventLoopProxy,
    webview::RpcRequest,
    webview::{WebView, WebViewBuilder},
};

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
    let mut cfg = DesktopConfig::new();
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
                    .with_url("dioxus://index.html/")
                    .unwrap()
                    .with_rpc_handler(move |_window: &Window, req: RpcRequest| {
                        match req.method.as_str() {
                            "user_event" => {
                                let event = events::trigger_from_serialized(req.params.unwrap());
                                log::trace!("User event: {:?}", event);
                                sender.unbounded_send(SchedulerMsg::Event(event)).unwrap();
                            }
                            "initialize" => {
                                is_ready.store(true, std::sync::atomic::Ordering::Relaxed);
                                let _ = proxy.send_event(UserWindowEvent::Update);
                            }
                            "browser_open" => {
                                let data = req.params.unwrap();
                                log::trace!("Open browser: {:?}", data);
                                if data.is_array() {
                                    let arr = data.as_array().unwrap();
                                    if arr[0].is_object() {
                                        let temp = arr[0].as_object().unwrap();
                                        if temp.contains_key("href") {
                                            let url = temp.get("href").unwrap().as_str().unwrap();
                                            match webbrowser::open(url) {
                                                Ok(_) => {}
                                                Err(e) => {
                                                    log::trace!("Open browser error: {:?}", e)
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        None
                    })
                    .with_custom_protocol("dioxus".into(), move |request| {
                        // Any content that that uses the `dioxus://` scheme will be shuttled through this handler as a "special case"
                        // For now, we only serve two pieces of content which get included as bytes into the final binary.
                        let path = request.uri().replace("dioxus://", "");

                        if path.trim_end_matches('/') == "index.html" {
                            wry::http::ResponseBuilder::new()
                                .mimetype("text/html")
                                .body(include_bytes!("./index.html").to_vec())
                        } else if path.trim_end_matches('/') == "index.html/index.js" {
                            wry::http::ResponseBuilder::new()
                                .mimetype("text/javascript")
                                .body(include_bytes!("./index.js").to_vec())
                        } else {
                            wry::http::ResponseBuilder::new()
                                .status(wry::http::status::StatusCode::NOT_FOUND)
                                .body(format!("Not found: {}", path).as_bytes().to_vec())
                        }
                    })
                    .with_file_drop_handler(move |window, evet| {
                        if let Some(handler) = file_handler.as_ref() {
                            return handler(window, evet);
                        }
                        false
                    });

                for (name, handler) in cfg.protocos.drain(..) {
                    webview = webview.with_custom_protocol(name, handler)
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

            Event::UserEvent(_evt) => {
                //
                match _evt {
                    UserWindowEvent::Update => desktop.try_load_ready_webviews(),
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

pub enum UserWindowEvent {
    Update,
}

pub struct DesktopController {
    pub proxy: EventLoopProxy<UserWindowEvent>,
    pub webviews: HashMap<WindowId, WebView>,
    pub sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
    pub pending_edits: Arc<RwLock<VecDeque<String>>>,
    pub quit_app_on_close: bool,
    pub is_ready: Arc<AtomicBool>,
}

impl DesktopController {
    // Launch the virtualdom on its own thread managed by tokio
    // returns the desktop state
    pub fn new_on_tokio<P: Send + 'static>(
        root: Component<P>,
        props: P,
        evt: EventLoopProxy<UserWindowEvent>,
    ) -> Self {
        let edit_queue = Arc::new(RwLock::new(VecDeque::new()));
        let pending_edits = edit_queue.clone();

        let (sender, receiver) = futures_channel::mpsc::unbounded::<SchedulerMsg>();
        let return_sender = sender.clone();
        let proxy = evt.clone();

        std::thread::spawn(move || {
            // We create the runtime as multithreaded, so you can still "spawn" onto multiple threads
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async move {
                let mut dom =
                    VirtualDom::new_with_props_and_scheduler(root, props, (sender, receiver));

                let edits = dom.rebuild();

                edit_queue
                    .write()
                    .unwrap()
                    .push_front(serde_json::to_string(&edits.edits).unwrap());

                loop {
                    dom.wait_for_work().await;
                    let mut muts = dom.work_with_deadline(|| false);

                    while let Some(edit) = muts.pop() {
                        edit_queue
                            .write()
                            .unwrap()
                            .push_front(serde_json::to_string(&edit.edits).unwrap());
                    }

                    let _ = evt.send_event(UserWindowEvent::Update);
                }
            })
        });

        Self {
            pending_edits,
            sender: return_sender,
            proxy,
            webviews: HashMap::new(),
            is_ready: Arc::new(AtomicBool::new(false)),
            quit_app_on_close: true,
        }
    }

    pub fn close_window(&mut self, window_id: WindowId, control_flow: &mut ControlFlow) {
        self.webviews.remove(&window_id);

        if self.webviews.is_empty() && self.quit_app_on_close {
            *control_flow = ControlFlow::Exit;
        }
    }

    pub fn try_load_ready_webviews(&mut self) {
        if self.is_ready.load(std::sync::atomic::Ordering::Relaxed) {
            let mut queue = self.pending_edits.write().unwrap();
            let (_id, view) = self.webviews.iter_mut().next().unwrap();

            while let Some(edit) = queue.pop_back() {
                view.evaluate_script(&format!("window.interpreter.handleEdits({})", edit))
                    .unwrap();
            }
        } else {
            println!("waiting for ready");
        }
    }
}
