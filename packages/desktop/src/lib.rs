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

mod eval;
mod waker;
mod webview;

pub use cfg::Config;
use desktop_context::UserWindowEvent;
pub use desktop_context::{use_window, DesktopContext};
use dioxus_core::*;
use dioxus_html::HtmlEvent;
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
pub fn launch_with_props<P: 'static + Send>(root: Component<P>, props: P, mut cfg: Config) {
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
    let mut webviews = HashMap::new();

    event_loop.run(move |window_event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match window_event {
            Event::NewEvents(StartCause::Init) => {
                let (eval_sender, eval_reciever) = tokio::sync::mpsc::unbounded_channel();
                let window = Rc::new(webview::build(&mut cfg, event_loop, proxy.clone()));
                let ctx = DesktopContext::new(window.clone(), proxy.clone(), eval_reciever);
                dom.base_scope().provide_context(ctx);
                webviews.insert(window.window().id(), window.clone());
                proxy.send_event(UserWindowEvent::Poll).unwrap();
            }

            Event::MainEventsCleared => {}
            Event::Resumed => {}
            Event::Suspended => {}
            Event::LoopDestroyed => {}
            Event::RedrawRequested(_id) => {}

            Event::NewEvents(cause) => {}

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Destroyed { .. } => {
                    // desktop.close_window(window_id, control_flow);
                }
                _ => {}
            },

            Event::UserEvent(UserWindowEvent::Initialize) => {
                send_edits(dom.rebuild(), &mut webviews);
            }

            Event::UserEvent(UserWindowEvent::CloseWindow) => *control_flow = ControlFlow::Exit,

            Event::UserEvent(UserWindowEvent::EvalResult(_)) => todo!(),

            Event::UserEvent(UserWindowEvent::UserEvent(json_value)) => {
                let evt = match serde_json::from_value::<HtmlEvent>(json_value) {
                    Ok(value) => value,
                    Err(_) => return,
                };

                dom.handle_event(&evt.name, evt.data.into_any(), evt.element, evt.bubbles);

                send_edits(dom.render_immediate(), &mut webviews);
            }

            Event::UserEvent(UserWindowEvent::Poll) => {
                poll_vdom(&waker, &mut dom, &mut webviews);
            }

            _ => todo!(),
        }
    })
}

type Webviews = HashMap<tao::window::WindowId, Rc<wry::webview::WebView>>;

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

fn send_edits(edits: Mutations, webviews: &mut Webviews) {
    let serialized = serde_json::to_string(&edits).unwrap();
    let (_id, view) = webviews.iter_mut().next().unwrap();
    _ = view.evaluate_script(&format!("window.interpreter.handleEdits({})", serialized));
}
