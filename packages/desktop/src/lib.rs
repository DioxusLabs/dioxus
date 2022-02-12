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
pub mod desktop_context;
pub mod escape;
pub mod events;

use cfg::DesktopConfig;
pub use desktop_context::use_window;
use desktop_context::DesktopContext;
use dioxus_core::*;
use futures::future::poll_fn;
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
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
    application::{event_loop::EventLoopProxy, window::Fullscreen},
    webview::RpcRequest,
    webview::{WebView, WebViewBuilder},
};

/// Launch the app but without windows
///
/// Manually spawn in windows later
pub fn launch_without_windows() {}

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
    let mut cfg = DesktopConfig::default();
    builder(&mut cfg);

    let event_loop = EventLoop::with_user_event();

    let (s_tx, s_rx) = tokio::sync::mpsc::unbounded_channel();

    /*
    Note: all webviews are created on the same thread.

    This should be okay (performance wise) because most users won't need mulithreading
    across webviews.

    We trade off performance for a gain in ergonomics between VDoms, since we can
    update props from one webview to another.
    */
    std::thread::spawn(move || {
        // We create the runtime as multithreaded, so you can still "spawn" onto multiple threads
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        // maintain all the webviews in a task that we poll
        let futures = HashMap::<WindowId, Pin<Box<dyn Future<Output = ()>>>>::new();

        runtime.block_on(async move {
            use futures_util::future::{select, Either};

            let mut to_remove = vec![];
            loop {
                let poll_webviews = poll_fn(|cx| {
                    for (id, fut) in futures {
                        let stat = fut.as_mut().poll(cx);

                        // uh, I mean this shouldn't really happen, but yeah okay
                        if stat.is_ready() {
                            to_remove.push(id);
                        }
                    }
                    //
                    std::task::Poll::Pending
                });

                let poll_channel = s_rx.recv();

                match select(poll_webviews, poll_channel).await {
                    Either::Left((_, _)) => {}
                    Either::Right((msg, _)) => self.pending_messages.push_front(msg.unwrap()),
                }
            }
        })
    });

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
                                let _ = proxy.send_event(UserWindowEvent {
                                    event: UserWindowEventType::Update,
                                    window_id,
                                });
                            }
                            "browser_open" => {
                                let data = req.params.unwrap();
                                log::trace!("Open browser: {:?}", data);
                                if let Some(arr) = data.as_array() {
                                    if let Some(temp) = arr[0].as_object() {
                                        if temp.contains_key("href") {
                                            let url = temp.get("href").unwrap().as_str().unwrap();
                                            if let Err(e) = webbrowser::open(url) {
                                                log::error!("Open Browser error: {:?}", e);
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        None
                    })
                    .with_custom_protocol(String::from("dioxus"), move |request| {
                        // Any content that that uses the `dioxus://` scheme will be shuttled through this handler as a "special case"
                        // For now, we only serve two pieces of content which get included as bytes into the final binary.
                        let path = request.uri().replace("dioxus://", "");

                        // all assets shouldbe called from index.html
                        let trimmed = path.trim_start_matches("index.html/");

                        if trimmed.is_empty() {
                            wry::http::ResponseBuilder::new()
                                .mimetype("text/html")
                                .body(include_bytes!("./index.html").to_vec())
                        } else if trimmed == "index.js" {
                            wry::http::ResponseBuilder::new()
                                .mimetype("text/javascript")
                                .body(dioxus_interpreter_js::INTERPRTER_JS.as_bytes().to_vec())
                        } else {
                            // Read the file content from file path
                            use std::fs::read;

                            let path_buf = std::path::Path::new(trimmed).canonicalize()?;
                            let cur_path = std::path::Path::new(".").canonicalize()?;

                            if !path_buf.starts_with(cur_path) {
                                return wry::http::ResponseBuilder::new()
                                    .status(wry::http::status::StatusCode::FORBIDDEN)
                                    .body(String::from("Forbidden").into_bytes());
                            }

                            if !path_buf.exists() {
                                return wry::http::ResponseBuilder::new()
                                    .status(wry::http::status::StatusCode::NOT_FOUND)
                                    .body(String::from("Not Found").into_bytes());
                            }

                            // todo: try to canonicalize the path if we're instead a binary

                            let mime = mime_guess::from_path(&path_buf).first_or_octet_stream();

                            // do not let path searching to go two layers beyond the caller level
                            let data = read(path_buf)?;
                            let meta = format!("{}", mime);

                            wry::http::ResponseBuilder::new().mimetype(&meta).body(data)
                        }
                    })
                    .with_file_drop_handler(move |window, evet| {
                        file_handler
                            .as_ref()
                            .map(|handler| handler(window, evet))
                            .unwrap_or_default()
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

            Event::UserEvent(UserWindowEvent { window_id, event }) => {
                let webview = desktop.webviews.get(&window_id).unwrap();
                let window = webview.window();
                match event {
                    UserWindowEventType::Update => desktop.try_load_ready_webviews(),
                    UserWindowEventType::CloseWindow => {
                        // todo: close the window, not the app
                        *control_flow = ControlFlow::Exit;
                    }
                    UserWindowEventType::NewWindow(id) => todo!(),

                    UserWindowEventType::Visible(state) => window.set_visible(state),
                    UserWindowEventType::Minimize(state) => window.set_minimized(state),
                    UserWindowEventType::Maximize(state) => window.set_maximized(state),
                    UserWindowEventType::FocusWindow => window.set_focus(),
                    UserWindowEventType::Resizable(state) => window.set_resizable(state),
                    UserWindowEventType::AlwaysOnTop(state) => window.set_always_on_top(state),
                    UserWindowEventType::CursorVisible(state) => window.set_cursor_visible(state),
                    UserWindowEventType::SetTitle(content) => window.set_title(&content),
                    UserWindowEventType::SetDecorations(state) => window.set_decorations(state),

                    UserWindowEventType::Fullscreen(state) => {
                        let current_monitor = window.current_monitor();

                        if current_monitor.is_none() {
                            return;
                        }

                        let fullscreen = if state {
                            Some(Fullscreen::Borderless(current_monitor))
                        } else {
                            None
                        };

                        window.set_fullscreen(fullscreen);
                    }

                    UserWindowEventType::DragWindow => {
                        // start to drag the window.
                        // if the drag_window have any err. we don't do anything.
                        if window.fullscreen().is_some() {
                            return;
                        }
                        let _ = window.drag_window();
                    }
                    UserWindowEventType::CursorGrab(state) => {
                        let _ = window.set_cursor_grab(state);
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

pub enum WebviewManagement {
    Open {
        make: Box<dyn FnOnce() -> VirtualDom>,
    },
}

struct UserWindowEvent {
    window_id: WindowId,
    event: UserWindowEventType,
}

pub enum UserWindowEventType {
    Update,
    DragWindow,
    CloseWindow,
    FocusWindow,
    Visible(bool),
    Minimize(bool),
    Maximize(bool),
    Resizable(bool),
    AlwaysOnTop(bool),
    Fullscreen(bool),
    CursorVisible(bool),
    CursorGrab(bool),
    SetTitle(String),
    SetDecorations(bool),

    NewWindow(WindowId),
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
        window_id: WindowId,
    ) -> Self {
        let edit_queue = Arc::new(RwLock::new(VecDeque::new()));
        let pending_edits = edit_queue.clone();

        let (sender, receiver) = futures_channel::mpsc::unbounded::<SchedulerMsg>();
        let return_sender = sender.clone();
        let proxy = evt.clone();

        let desktop_context_proxy = proxy.clone();
        std::thread::spawn(move || {
            // We create the runtime as multithreaded, so you can still "spawn" onto multiple threads
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            let futures = HashMap::<WindowId, Box<dyn Future<Output = ()>>>::new();

            runtime.block_on(async move {
                //
                // wait for signal to come in

                // poll virtualdoms

                task.await;
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

// let task = async move {
//     let mut dom =
//         VirtualDom::new_with_props_and_scheduler(root, props, (sender, receiver));

//     let window_context = DesktopContext::new(desktop_context_proxy);

//     dom.base_scope().provide_context(window_context);

//     let edits = dom.rebuild();

//     edit_queue
//         .write()
//         .unwrap()
//         .push_front(serde_json::to_string(&edits.edits).unwrap());

//     loop {
//         dom.wait_for_work().await;

//         let mut muts = dom.work_with_deadline(|| false);

//         while let Some(edit) = muts.pop() {
//             edit_queue
//                 .write()
//                 .unwrap()
//                 .push_front(serde_json::to_string(&edit.edits).unwrap());
//         }

//         let _ = evt.send_event(UserWindowEvent {
//             event: UserWindowEventType::Update,
//             window_id,
//         });
//     }
// };
