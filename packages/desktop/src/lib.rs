#![doc = include_str!("readme.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub mod cfg;
pub mod desktop_context;
pub mod escape;
pub mod events;

use cfg::DesktopConfig;
pub use desktop_context::use_window;
use desktop_context::DesktopContext;
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
    application::{event_loop::EventLoopProxy, window::Fullscreen},
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
                                    println!("browser_open");
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
                    UserWindowEvent::DragWindow => {
                        // this loop just run once, because dioxus-desktop is unsupport multi-window.
                        for webview in desktop.webviews.values() {
                            let window = webview.window();
                            // start to drag the window.
                            // if the drag_window have any err. we don't do anything.

                            if window.fullscreen().is_some() {
                                return;
                            }

                            let _ = window.drag_window();
                        }
                    }
                    UserWindowEvent::CloseWindow => {
                        // close window
                        *control_flow = ControlFlow::Exit;
                    }
                    UserWindowEvent::Visible(state) => {
                        for webview in desktop.webviews.values() {
                            let window = webview.window();
                            window.set_visible(state);
                        }
                    }
                    UserWindowEvent::Minimize(state) => {
                        // this loop just run once, because dioxus-desktop is unsupport multi-window.
                        for webview in desktop.webviews.values() {
                            let window = webview.window();
                            // change window minimized state.
                            window.set_minimized(state);
                        }
                    }
                    UserWindowEvent::Maximize(state) => {
                        // this loop just run once, because dioxus-desktop is unsupport multi-window.
                        for webview in desktop.webviews.values() {
                            let window = webview.window();
                            // change window maximized state.
                            window.set_maximized(state);
                        }
                    }
                    UserWindowEvent::Fullscreen(state) => {
                        for webview in desktop.webviews.values() {
                            let window = webview.window();

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
                    }
                    UserWindowEvent::FocusWindow => {
                        for webview in desktop.webviews.values() {
                            let window = webview.window();
                            window.set_focus();
                        }
                    }
                    UserWindowEvent::Resizable(state) => {
                        for webview in desktop.webviews.values() {
                            let window = webview.window();
                            window.set_resizable(state);
                        }
                    }
                    UserWindowEvent::AlwaysOnTop(state) => {
                        for webview in desktop.webviews.values() {
                            let window = webview.window();
                            window.set_always_on_top(state);
                        }
                    }

                    UserWindowEvent::CursorVisible(state) => {
                        for webview in desktop.webviews.values() {
                            let window = webview.window();
                            window.set_cursor_visible(state);
                        }
                    }
                    UserWindowEvent::CursorGrab(state) => {
                        for webview in desktop.webviews.values() {
                            let window = webview.window();
                            let _ = window.set_cursor_grab(state);
                        }
                    }

                    UserWindowEvent::SetTitle(content) => {
                        for webview in desktop.webviews.values() {
                            let window = webview.window();
                            window.set_title(&content);
                        }
                    }
                    UserWindowEvent::SetDecorations(state) => {
                        for webview in desktop.webviews.values() {
                            let window = webview.window();
                            window.set_decorations(state);
                        }
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

pub enum UserWindowEvent {
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

        let desktop_context_proxy = proxy.clone();
        std::thread::spawn(move || {
            // We create the runtime as multithreaded, so you can still "spawn" onto multiple threads
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async move {
                let mut dom =
                    VirtualDom::new_with_props_and_scheduler(root, props, (sender, receiver));

                let window_context = DesktopContext::new(desktop_context_proxy);

                dom.base_scope().provide_context(window_context);

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
