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
mod waker;
mod webview;

#[cfg(all(feature = "hot-reload", debug_assertions))]
mod hot_reload;

pub use cfg::Config;
use desktop_context::UserWindowEvent;
pub use desktop_context::{use_window, DesktopContext};
use dioxus_core::*;
use dioxus_html::HtmlEvent;
pub use eval::{use_eval, EvalResult};
use futures_util::{pin_mut, FutureExt};
use std::collections::HashMap;
use std::rc::Rc;
use std::task::Waker;
pub use tao::dpi::{LogicalSize, PhysicalSize};
pub use tao::window::WindowBuilder;
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
pub use wry;
pub use wry::application as tao;

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
pub fn launch_with_props<P: 'static>(root: Component<P>, props: P, mut cfg: Config) {
    let mut dom = VirtualDom::new_with_props(root, props);

    let event_loop = EventLoop::with_user_event();

    let proxy = event_loop.create_proxy();

    // We start the tokio runtime *on this thread*
    // Any future we poll later will use this runtime to spawn tasks and for IO
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // We enter the runtime but we poll futures manually, circumventing the per-task runtime budget
    let _guard = rt.enter();

    // We want to poll the virtualdom and the event loop at the same time, so the waker will be connected to both
    let waker = waker::tao_waker(&proxy);

    // We only have one webview right now, but we'll have more later
    // Store them in a hashmap so we can remove them when they're closed
    let mut webviews = HashMap::new();

    event_loop.run(move |window_event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match window_event {
            Event::UserEvent(UserWindowEvent::CloseWindow) => *control_flow = ControlFlow::Exit,

            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Destroyed { .. } => {
                    webviews.remove(&window_id);

                    if webviews.is_empty() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                _ => {}
            },

            Event::NewEvents(StartCause::Init) => {
                let window = webview::build(&mut cfg, event_loop, proxy.clone());

                dom.base_scope()
                    .provide_context(DesktopContext::new(window.clone(), proxy.clone()));

                webviews.insert(window.window().id(), window);

                _ = proxy.send_event(UserWindowEvent::Poll);
            }

            Event::UserEvent(UserWindowEvent::Poll) => {
                poll_vdom(&waker, &mut dom, &mut webviews);
            }

            Event::UserEvent(UserWindowEvent::Ipc(msg)) if msg.method() == "user_event" => {
                let evt = match serde_json::from_value::<HtmlEvent>(msg.params()) {
                    Ok(value) => value,
                    Err(_) => return,
                };

                dom.handle_event(&evt.name, evt.data.into_any(), evt.element, evt.bubbles);

                send_edits(dom.render_immediate(), &mut webviews);
            }

            Event::UserEvent(UserWindowEvent::Ipc(msg)) if msg.method() == "initialize" => {
                send_edits(dom.rebuild(), &mut webviews);
            }

            // When the webview chirps back with the result of the eval, we send it to the active receiver
            //
            // This currently doesn't perform any targeting to the callsite, so if you eval multiple times at once,
            // you might the wrong result. This should be fixed
            Event::UserEvent(UserWindowEvent::Ipc(msg)) if msg.method() == "eval_result" => {
                dom.base_scope()
                    .consume_context::<DesktopContext>()
                    .unwrap()
                    .eval
                    .send(msg.params())
                    .unwrap();
            }

            Event::UserEvent(UserWindowEvent::Ipc(msg)) if msg.method() == "browser_open" => {
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
        }
    })
}

type Webviews = HashMap<tao::window::WindowId, Rc<wry::webview::WebView>>;

/// Poll the virtualdom until it's pending
///
/// The waker we give it is connected to the event loop, so it will wake up the event loop when it's ready to be polled again
///
/// All IO is done on the tokio runtime we started earlier
fn poll_vdom(waker: &Waker, dom: &mut VirtualDom, webviews: &mut Webviews) {
    let mut cx = std::task::Context::from_waker(waker);

    loop {
        {
            let fut = dom.wait_for_work();
            pin_mut!(fut);

            match fut.poll_unpin(&mut cx) {
                std::task::Poll::Ready(_) => {}
                std::task::Poll::Pending => break,
            }
        }

        send_edits(dom.render_immediate(), webviews);
    }
}

/// Send a list of mutations to the webview
fn send_edits(edits: Mutations, webviews: &mut Webviews) {
    let serialized = serde_json::to_string(&edits).unwrap();

    let (_id, view) = webviews.iter_mut().next().unwrap();

    // todo: use SSE and binary data to send the edits with lower overhead
    _ = view.evaluate_script(&format!("window.interpreter.handleEdits({})", serialized));
}
