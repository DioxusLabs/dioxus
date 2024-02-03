use crate::{
    app::App,
    ipc::{EventData, IpcMethod, UserWindowEvent},
    Config,
};
use dioxus_core::*;
use tao::event::{Event, StartCause, WindowEvent};

/// Launch the WebView and run the event loop.
///
/// This function will start a multithreaded Tokio runtime as well the WebView event loop.
///
/// ```rust, no_run
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
/// ```rust, no_run
/// use dioxus::prelude::*;
/// use dioxus_desktop::*;
///
/// fn main() {
///     dioxus_desktop::launch_cfg(app, Config::default().with_window(WindowBuilder::new().with_title("My App")));
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
/// If the [`tokio`] feature is enabled, this will also startup and block a tokio runtime using the unconstrained task.
/// This function will start a multithreaded Tokio runtime as well the WebView event loop. This will block the current thread.
///
/// You can configure the WebView window with a configuration closure
///
/// ```rust, no_run
/// use dioxus::prelude::*;
/// use dioxus_desktop::Config;
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
    #[cfg(feature = "tokio")]
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(tokio::task::unconstrained(async move {
            launch_with_props_blocking(root, props, cfg);
        }));

    #[cfg(not(feature = "tokio"))]
    launch_with_props_blocking(root, props, cfg);
}

/// Launch the WebView and run the event loop, with configuration and root props.
///
/// This will block the main thread, and *must* be spawned on the main thread. This function does not assume any runtime
/// and is equivalent to calling launch_with_props with the tokio feature disabled.
pub fn launch_with_props_blocking<P: 'static>(root: Component<P>, props: P, cfg: Config) {
    let (event_loop, mut app) = App::new(cfg, props, root);

    event_loop.run(move |window_event, _, control_flow| {
        app.tick(&window_event);

        match window_event {
            Event::NewEvents(StartCause::Init) => app.handle_start_cause_init(),
            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => app.handle_close_requested(window_id),
                WindowEvent::Destroyed { .. } => app.window_destroyed(window_id),
                _ => {}
            },
            Event::UserEvent(UserWindowEvent(event, id)) => match event {
                EventData::Poll => app.poll_vdom(id),
                EventData::NewWindow => app.handle_new_window(),
                EventData::CloseWindow => app.handle_close_msg(id),
                #[cfg(all(feature = "hot-reload", debug_assertions))]
                EventData::HotReloadEvent(msg) => app.handle_hot_reload_msg(msg),
                EventData::Ipc(msg) => match msg.method() {
                    IpcMethod::FileDialog => app.handle_file_dialog_msg(msg, id),
                    IpcMethod::UserEvent => app.handle_user_event_msg(msg, id),
                    IpcMethod::Query => app.handle_query_msg(msg, id),
                    IpcMethod::BrowserOpen => app.handle_browser_open(msg),
                    IpcMethod::Initialize => app.handle_initialize_msg(id),
                    IpcMethod::Other(_) => {}
                },
            },
            _ => {}
        }

        *control_flow = app.control_flow;
    })
}
