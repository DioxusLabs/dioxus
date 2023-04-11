#![doc = include_str!("readme.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]

mod cfg;
mod desktop_context;
mod escape;
mod eval;
mod events;
mod protocol;
mod shortcut;
mod waker;
mod webview;

pub use cfg::Config;
pub use desktop_context::{
    use_window, use_wry_event_handler, DesktopService, WryEventHandler, WryEventHandlerId,
};
use desktop_context::{
    DesktopContext, EventData, UserWindowEvent, WebviewQueue, WindowEventHandlers,
};
use dioxus_core::*;
use dioxus_html::HtmlEvent;
pub use eval::{use_eval, EvalResult};
use futures_util::{pin_mut, FutureExt};
use shortcut::ShortcutRegistry;
pub use shortcut::{use_global_shortcut, ShortcutHandle, ShortcutId, ShortcutRegistryError};
use std::collections::HashMap;
use std::rc::Rc;
use std::task::Waker;
pub use tao::dpi::{LogicalSize, PhysicalSize};
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
pub use tao::window::WindowBuilder;
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
pub use wry;
pub use wry::application as tao;
use wry::application::window::WindowId;
use wry::webview::WebView;

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
    let event_loop = EventLoop::<UserWindowEvent>::with_user_event();

    let proxy = event_loop.create_proxy();

    // Intialize hot reloading if it is enabled
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    {
        let proxy = proxy.clone();
        dioxus_hot_reload::connect(move |template| {
            let _ = proxy.send_event(UserWindowEvent(
                EventData::HotReloadEvent(template),
                unsafe { WindowId::dummy() },
            ));
        });
    }

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

    let shortcut_manager = ShortcutRegistry::new(&event_loop);

    let web_view = create_new_window(
        cfg,
        &event_loop,
        &proxy,
        VirtualDom::new_with_props(root, props),
        &queue,
        &event_handlers,
        shortcut_manager.clone(),
    );

    // By default, we'll create a new window when the app starts
    queue.borrow_mut().push(web_view);

    event_loop.run(move |window_event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        event_handlers.apply_event(&window_event, event_loop);

        match window_event {
            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => {
                    webviews.remove(&window_id);

                    if webviews.is_empty() {
                        *control_flow = ControlFlow::Exit
                    }
                }
                WindowEvent::Destroyed { .. } => {
                    webviews.remove(&window_id);

                    if webviews.is_empty() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                _ => {}
            },

            Event::NewEvents(StartCause::Init)
            | Event::UserEvent(UserWindowEvent(EventData::NewWindow, _)) => {
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
                    let evt = match serde_json::from_value::<HtmlEvent>(msg.params()) {
                        Ok(value) => value,
                        Err(_) => return,
                    };

                    let view = webviews.get_mut(&event.1).unwrap();

                    view.dom
                        .handle_event(&evt.name, evt.data.into_any(), evt.element, evt.bubbles);

                    send_edits(view.dom.render_immediate(), &view.desktop_context.webview);
                }

                EventData::Ipc(msg) if msg.method() == "initialize" => {
                    let view = webviews.get_mut(&event.1).unwrap();
                    send_edits(view.dom.rebuild(), &view.desktop_context.webview);
                }

                // When the webview chirps back with the result of the eval, we send it to the active receiver
                //
                // This currently doesn't perform any targeting to the callsite, so if you eval multiple times at once,
                // you might the wrong result. This should be fixed
                EventData::Ipc(msg) if msg.method() == "eval_result" => {
                    webviews[&event.1]
                        .dom
                        .base_scope()
                        .consume_context::<DesktopContext>()
                        .unwrap()
                        .eval
                        .send(msg.params())
                        .unwrap();
                }

                EventData::Ipc(msg) if msg.method() == "browser_open" => {
                    if let Some(temp) = msg.params().as_object() {
                        if temp.contains_key("href") {
                            let open = webbrowser::open(temp["href"].as_str().unwrap());
                            if let Err(e) = open {
                                log::error!("Open Browser error: {:?}", e);
                            }
                        }
                    }
                }

                _ => {}
            },
            Event::GlobalShortcutEvent(id) => shortcut_manager.call_handlers(id),
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
    let webview = webview::build(&mut cfg, event_loop, proxy.clone());
    let desktop_context = Rc::from(DesktopService::new(
        webview,
        proxy.clone(),
        event_loop.clone(),
        queue.clone(),
        event_handlers.clone(),
        shortcut_manager,
    ));

    dom.base_scope().provide_context(desktop_context.clone());

    let id = desktop_context.webview.window().id();

    // We want to poll the virtualdom and the event loop at the same time, so the waker will be connected to both

    WebviewHandler {
        desktop_context,
        dom,
        waker: waker::tao_waker(proxy, id),
    }
}

struct WebviewHandler {
    dom: VirtualDom,
    desktop_context: DesktopContext,
    waker: Waker,
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
