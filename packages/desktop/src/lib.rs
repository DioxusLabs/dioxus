#![doc = include_str!("readme.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]

mod app;
mod asset_handler;
mod cfg;
mod desktop_context;
mod element;
mod escape;
mod eval;
mod events;
mod file_upload;
mod hooks;
#[cfg(any(target_os = "ios", target_os = "android"))]
mod mobile_shortcut;
mod protocol;
mod query;
mod shortcut;
mod waker;
mod webview;

pub use cfg::{Config, WindowCloseBehaviour};
pub use desktop_context::use_window;
pub use desktop_context::DesktopContext;
#[allow(deprecated)]
pub use desktop_context::{
    use_wry_event_handler, window, DesktopService, WryEventHandler, WryEventHandlerId,
};
use desktop_context::{EventData, UserWindowEvent};
use dioxus_core::*;
use events::IpcMethod;
pub use protocol::{use_asset_handler, AssetFuture, AssetHandler, AssetRequest, AssetResponse};
pub use shortcut::{use_global_shortcut, ShortcutHandle, ShortcutId, ShortcutRegistryError};
pub use tao;
pub use tao::dpi::{LogicalSize, PhysicalSize};
use tao::event::{Event, StartCause, WindowEvent};
pub use tao::window::WindowBuilder;
use tokio::runtime::Builder;
pub use webview::build_default_menu_bar;
pub use wry;

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
    // We start the tokio runtime *on this thread*
    // Any future we poll later will use this runtime to spawn tasks and for IO
    // I would love to just allow dioxus to work with any runtime... but tokio is weird
    let rt = &Builder::new_multi_thread().enable_all().build().unwrap();
    let _guard = rt.enter();

    let (event_loop, mut app) = app::App::new(cfg, props, root);

    event_loop.run(move |window_event, event_loop, control_flow| {
        app.tick(&window_event, event_loop);

        match window_event {
            Event::NewEvents(StartCause::Init) => app.handle_start_cause_init(event_loop),
            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => app.handle_close_requested(window_id),
                WindowEvent::Destroyed { .. } => app.window_destroyed(window_id),
                _ => {}
            },
            Event::UserEvent(UserWindowEvent(event, id)) => match event {
                EventData::Poll => app.handle_poll_msg(id),
                EventData::NewWindow => app.handle_new_window(),
                EventData::CloseWindow => app.handle_close_msg(id),
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
