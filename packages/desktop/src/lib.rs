#![doc = include_str!("readme.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]

mod cfg;
mod desktop_context;
mod element;
mod escape;
mod eval;
mod events;
mod file_upload;
#[cfg(any(target_os = "ios", target_os = "android"))]
mod mobile_shortcut;
mod protocol;
mod query;
mod shortcut;
mod waker;
mod webview;

use crate::query::QueryResult;
use crate::shortcut::GlobalHotKeyEvent;
pub use cfg::{Config, WindowCloseBehaviour};
pub use desktop_context::DesktopContext;
pub use desktop_context::{
    use_window, use_wry_event_handler, DesktopService, WryEventHandler, WryEventHandlerId,
};
use desktop_context::{EventData, UserWindowEvent, WebviewQueue, WindowEventHandlers};
use dioxus_core::*;
use dioxus_html::MountedData;
use dioxus_html::{native_bind::NativeFileEngine, FormData, HtmlEvent};
use element::DesktopElement;
use eval::init_eval;
use futures_util::{pin_mut, FutureExt};
use shortcut::ShortcutRegistry;
pub use shortcut::{use_global_shortcut, ShortcutHandle, ShortcutId, ShortcutRegistryError};
use std::cell::Cell;
use std::rc::Rc;
use std::task::Waker;
use std::{collections::HashMap, sync::Arc};
pub use tao::dpi::{LogicalSize, PhysicalSize};
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
pub use tao::window::WindowBuilder;
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::ControlFlow,
};
pub use wry;
pub use wry::application as tao;
use wry::application::event_loop::EventLoopBuilder;
use wry::webview::WebView;
use wry::{application::window::WindowId, webview::WebContext};

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
/// This function will start a multithreaded Tokio runtime as well the WebView event loop. This will block the current thread.
///
/// You can configure the WebView window with a configuration closure
///
/// ```rust, ignore
/// use dioxus::prelude::*;
///
/// fn main() {
///     dioxus_desktop::launch_with_props(app, AppProps { name: "asd" }, Config::default());
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
pub fn launch_with_props<P: 'static>(root: Component<P>, props: P, cfg: Config) {
    let event_loop = EventLoopBuilder::<UserWindowEvent>::with_user_event().build();

    let proxy = event_loop.create_proxy();

    let window_behaviour = cfg.last_window_close_behaviour;

    // Intialize hot reloading if it is enabled
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    dioxus_hot_reload::connect({
        let proxy = proxy.clone();
        move |template| {
            let _ = proxy.send_event(UserWindowEvent(
                EventData::HotReloadEvent(template),
                unsafe { WindowId::dummy() },
            ));
        }
    });

    // We start the tokio runtime *on this thread*
    // Any future we poll later will use this runtime to spawn tasks and for IO
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // We enter the runtime but we poll futures manually, circumventing the per-task runtime budget
    let _guard = rt.enter();

    // We only have one webview right now, but we'll have more later
    // Store them in a hashmap so we can remove them when they're closed
    let mut webviews = HashMap::<WindowId, WebviewHandler>::new();

    // We use this to allow dynamically adding and removing window event handlers
    let event_handlers = WindowEventHandlers::default();

    let queue = WebviewQueue::default();

    let shortcut_manager = ShortcutRegistry::new();
    let global_hotkey_channel = GlobalHotKeyEvent::receiver();

    // move the props into a cell so we can pop it out later to create the first window
    // iOS panics if we create a window before the event loop is started
    let props = Rc::new(Cell::new(Some(props)));
    let cfg = Rc::new(Cell::new(Some(cfg)));

    event_loop.run(move |window_event, event_loop, control_flow| {
        *control_flow = ControlFlow::Poll;

        event_handlers.apply_event(&window_event, event_loop);

        if let Ok(event) = global_hotkey_channel.try_recv() {
            shortcut_manager.call_handlers(event);
        }

        match window_event {
            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => match window_behaviour {
                    cfg::WindowCloseBehaviour::LastWindowExitsApp => {
                        webviews.remove(&window_id);

                        if webviews.is_empty() {
                            *control_flow = ControlFlow::Exit
                        }
                    }
                    cfg::WindowCloseBehaviour::LastWindowHides => {
                        let Some(webview) = webviews.get(&window_id) else {
                            return;
                        };
                        hide_app_window(&webview.desktop_context.webview);
                    }
                    cfg::WindowCloseBehaviour::CloseWindow => {
                        webviews.remove(&window_id);
                    }
                },
                WindowEvent::Destroyed { .. } => {
                    webviews.remove(&window_id);

                    if matches!(
                        window_behaviour,
                        cfg::WindowCloseBehaviour::LastWindowExitsApp
                    ) && webviews.is_empty()
                    {
                        *control_flow = ControlFlow::Exit
                    }
                }
                _ => {}
            },

            Event::NewEvents(StartCause::Init) => {
                let props = props.take().unwrap();
                let cfg = cfg.take().unwrap();

                // Create a dom
                let dom = VirtualDom::new_with_props(root, props);

                let handler = create_new_window(
                    cfg,
                    event_loop,
                    &proxy,
                    dom,
                    &queue,
                    &event_handlers,
                    shortcut_manager.clone(),
                );

                let id = handler.desktop_context.webview.window().id();
                webviews.insert(id, handler);
                _ = proxy.send_event(UserWindowEvent(EventData::Poll, id));
            }

            Event::UserEvent(UserWindowEvent(EventData::NewWindow, _)) => {
                for handler in queue.borrow_mut().drain(..) {
                    let id = handler.desktop_context.webview.window().id();
                    webviews.insert(id, handler);
                    _ = proxy.send_event(UserWindowEvent(EventData::Poll, id));
                }
            }

            Event::UserEvent(event) => match event.0 {
                #[cfg(all(feature = "hot-reload", debug_assertions))]
                EventData::HotReloadEvent(msg) => match msg {
                    dioxus_hot_reload::HotReloadMsg::UpdateTemplate(template) => {
                        for webview in webviews.values_mut() {
                            webview.dom.replace_template(template);

                            poll_vdom(webview);
                        }
                    }
                    dioxus_hot_reload::HotReloadMsg::Shutdown => {
                        *control_flow = ControlFlow::Exit;
                    }
                },

                EventData::CloseWindow => {
                    webviews.remove(&event.1);

                    if webviews.is_empty() {
                        *control_flow = ControlFlow::Exit
                    }
                }

                EventData::Poll => {
                    if let Some(view) = webviews.get_mut(&event.1) {
                        poll_vdom(view);
                    }
                }

                EventData::Ipc(msg) if msg.method() == "user_event" => {
                    let params = msg.params();

                    let evt = match serde_json::from_value::<HtmlEvent>(params) {
                        Ok(value) => value,
                        Err(_) => return,
                    };

                    let HtmlEvent {
                        element,
                        name,
                        bubbles,
                        data,
                    } = evt;

                    let view = webviews.get_mut(&event.1).unwrap();

                    // check for a mounted event placeholder and replace it with a desktop specific element
                    let as_any = if let dioxus_html::EventData::Mounted = &data {
                        let query = view
                            .dom
                            .base_scope()
                            .consume_context::<DesktopContext>()
                            .unwrap()
                            .query
                            .clone();

                        let element =
                            DesktopElement::new(element, view.desktop_context.clone(), query);

                        Rc::new(MountedData::new(element))
                    } else {
                        data.into_any()
                    };

                    view.dom.handle_event(&name, as_any, element, bubbles);

                    send_edits(view.dom.render_immediate(), &view.desktop_context.webview);
                }

                // When the webview sends a query, we need to send it to the query manager which handles dispatching the data to the correct pending query
                EventData::Ipc(msg) if msg.method() == "query" => {
                    let params = msg.params();

                    if let Ok(result) = serde_json::from_value::<QueryResult>(params) {
                        let view = webviews.get(&event.1).unwrap();
                        let query = view
                            .dom
                            .base_scope()
                            .consume_context::<DesktopContext>()
                            .unwrap()
                            .query
                            .clone();

                        query.send(result);
                    }
                }

                EventData::Ipc(msg) if msg.method() == "initialize" => {
                    let view = webviews.get_mut(&event.1).unwrap();
                    send_edits(view.dom.rebuild(), &view.desktop_context.webview);
                }

                EventData::Ipc(msg) if msg.method() == "browser_open" => {
                    if let Some(temp) = msg.params().as_object() {
                        if temp.contains_key("href") {
                            let open = webbrowser::open(temp["href"].as_str().unwrap());
                            if let Err(e) = open {
                                tracing::error!("Open Browser error: {:?}", e);
                            }
                        }
                    }
                }

                EventData::Ipc(msg) if msg.method() == "file_diolog" => {
                    if let Ok(file_diolog) =
                        serde_json::from_value::<file_upload::FileDialogRequest>(msg.params())
                    {
                        let id = ElementId(file_diolog.target);
                        let event_name = &file_diolog.event;
                        let event_bubbles = file_diolog.bubbles;
                        let files = file_upload::get_file_event(&file_diolog);
                        let data = Rc::new(FormData {
                            value: Default::default(),
                            values: Default::default(),
                            files: Some(Arc::new(NativeFileEngine::new(files))),
                        });

                        let view = webviews.get_mut(&event.1).unwrap();

                        if event_name == "change&input" {
                            view.dom
                                .handle_event("input", data.clone(), id, event_bubbles);
                            view.dom.handle_event("change", data, id, event_bubbles);
                        } else {
                            view.dom.handle_event(event_name, data, id, event_bubbles);
                        }

                        send_edits(view.dom.render_immediate(), &view.desktop_context.webview);
                    }
                }

                _ => {}
            },
            _ => {}
        }
    })
}

fn create_new_window(
    mut cfg: Config,
    event_loop: &EventLoopWindowTarget<UserWindowEvent>,
    proxy: &EventLoopProxy<UserWindowEvent>,
    dom: VirtualDom,
    queue: &WebviewQueue,
    event_handlers: &WindowEventHandlers,
    shortcut_manager: ShortcutRegistry,
) -> WebviewHandler {
    let (webview, web_context) = webview::build(&mut cfg, event_loop, proxy.clone());
    let desktop_context = Rc::from(DesktopService::new(
        webview,
        proxy.clone(),
        event_loop.clone(),
        queue.clone(),
        event_handlers.clone(),
        shortcut_manager,
    ));

    let cx = dom.base_scope();
    cx.provide_context(desktop_context.clone());

    // Init eval
    init_eval(cx);

    WebviewHandler {
        // We want to poll the virtualdom and the event loop at the same time, so the waker will be connected to both
        waker: waker::tao_waker(proxy, desktop_context.webview.window().id()),
        desktop_context,
        dom,
        _web_context: web_context,
    }
}

struct WebviewHandler {
    dom: VirtualDom,
    desktop_context: DesktopContext,
    waker: Waker,

    // Wry assumes the webcontext is alive for the lifetime of the webview.
    // We need to keep the webcontext alive, otherwise the webview will crash
    _web_context: WebContext,
}

/// Poll the virtualdom until it's pending
///
/// The waker we give it is connected to the event loop, so it will wake up the event loop when it's ready to be polled again
///
/// All IO is done on the tokio runtime we started earlier
fn poll_vdom(view: &mut WebviewHandler) {
    let mut cx = std::task::Context::from_waker(&view.waker);

    loop {
        {
            let fut = view.dom.wait_for_work();
            pin_mut!(fut);

            match fut.poll_unpin(&mut cx) {
                std::task::Poll::Ready(_) => {}
                std::task::Poll::Pending => break,
            }
        }

        send_edits(view.dom.render_immediate(), &view.desktop_context.webview);
    }
}

/// Send a list of mutations to the webview
fn send_edits(edits: Mutations, webview: &WebView) {
    let serialized = serde_json::to_string(&edits).unwrap();

    // todo: use SSE and binary data to send the edits with lower overhead
    _ = webview.evaluate_script(&format!("window.interpreter.handleEdits({serialized})"));
}

/// Different hide implementations per platform
#[allow(unused)]
fn hide_app_window(webview: &WebView) {
    #[cfg(target_os = "windows")]
    {
        use wry::application::platform::windows::WindowExtWindows;
        webview.window().set_visible(false);
        webview.window().set_skip_taskbar(true);
    }

    #[cfg(target_os = "linux")]
    {
        use wry::application::platform::unix::WindowExtUnix;
        webview.window().set_visible(false);
    }

    #[cfg(target_os = "macos")]
    {
        // webview.window().set_visible(false); has the wrong behaviour on macOS
        // It will hide the window but not show it again when the user switches
        // back to the app. `NSApplication::hide:` has the correct behaviour
        use objc::runtime::Object;
        use objc::{msg_send, sel, sel_impl};
        objc::rc::autoreleasepool(|| unsafe {
            let app: *mut Object = msg_send![objc::class!(NSApplication), sharedApplication];
            let nil = std::ptr::null_mut::<Object>();
            let _: () = msg_send![app, hide: nil];
        });
    }
}
