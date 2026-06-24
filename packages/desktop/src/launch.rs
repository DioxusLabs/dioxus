use crate::{
    Config, WindowConfig,
    app::App,
    ipc::{IpcMethod, UserWindowEvent},
};
use dioxus_core::*;
use dioxus_core_macro::rsx;
use std::{any::Any, cell::RefCell, rc::Rc};
use tao::event::{Event, StartCause, WindowEvent};

#[derive(Clone, dioxus_core_macro::Props)]
struct DesktopRootProps {
    root: fn() -> Element,
    config: Rc<RefCell<Option<WindowConfig>>>,
}

impl PartialEq for DesktopRootProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

#[derive(Clone, dioxus_core_macro::Props)]
struct LaunchedRootProps {
    root: fn() -> Element,
}

impl PartialEq for LaunchedRootProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

#[allow(non_snake_case)]
fn LaunchedRoot(props: LaunchedRootProps) -> Element {
    (props.root)()
}

#[allow(non_snake_case)]
fn WindowedRoot(props: DesktopRootProps) -> Element {
    let root = props.root;
    let config = crate::window_component::InitialWindowConfig::from_cell(props.config.clone());
    let launched_root = Element::Ok(
        <LaunchedRootProps as Properties>::component_builder(LaunchedRoot)
            .root(root)
            .build()
            .into_vnode(),
    );
    rsx! {
        crate::Window {
            config,
            {launched_root}
        }
    }
}

/// Run a desktop [`VirtualDom`] directly.
///
/// This raw entrypoint does not create an implicit native window. The root component must render
/// at least one [`Window`](crate::Window) component if the app should show UI. Use
/// [`launch`](crate::launch::launch) or `dioxus::LaunchBuilder::desktop().launch(...)` for the
/// normal API that wraps the user component in a default window.
///
/// This will block the main thread, and *must* be spawned on the main thread. This function does not assume any runtime
/// and is equivalent to calling launch_with_props with the tokio feature disabled.
pub fn launch_virtual_dom_blocking(virtual_dom: VirtualDom, mut desktop_config: Config) -> ! {
    let mut custom_event_handler = desktop_config.custom_event_handler.take();
    let (event_loop, mut app) = App::new(desktop_config, virtual_dom);

    event_loop.run(move |window_event, event_loop, control_flow| {
        // Set the control flow and check if any events need to be handled in the app itself
        app.tick(&window_event);

        if let Some(ref mut f) = custom_event_handler {
            f(&window_event, event_loop)
        }

        match window_event {
            Event::NewEvents(StartCause::Init) => app.handle_start_cause_init(),
            Event::LoopDestroyed => app.handle_loop_destroyed(),
            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => app.handle_close_requested(window_id),
                WindowEvent::Destroyed { .. } => app.window_destroyed(window_id),
                WindowEvent::Resized(new_size) => app.resize_window(window_id, new_size),
                _ => {}
            },

            Event::UserEvent(event) => match event {
                UserWindowEvent::Poll => app.poll_vdom(),
                UserWindowEvent::NewWindow => app.handle_new_window(),
                UserWindowEvent::RequestWindowClose(id) => app.handle_close_requested(id),
                UserWindowEvent::DestroyWindow(id) => app.destroy_window(id),
                UserWindowEvent::Shutdown => app.control_flow = tao::event_loop::ControlFlow::Exit,

                #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
                UserWindowEvent::GlobalHotKeyEvent(evnt) => app.handle_global_hotkey(evnt),

                #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
                UserWindowEvent::MudaMenuEvent(evnt) => app.handle_menu_event(evnt),

                #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
                UserWindowEvent::TrayMenuEvent(evnt) => app.handle_tray_menu_event(evnt),

                #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
                UserWindowEvent::TrayIconEvent(evnt) => app.handle_tray_icon_event(evnt),

                #[cfg(all(feature = "devtools", debug_assertions))]
                UserWindowEvent::HotReloadEvent(msg) => app.handle_hot_reload_msg(msg),

                // Windows-only drag-n-drop fix events. We need to call the interpreter drag-n-drop code.
                UserWindowEvent::WindowsDragDrop(id) => {
                    if let Some(app_webview) = app.webviews.get(&id) {
                        _ = app_webview
                            .desktop_context
                            .webview
                            .evaluate_script("window.interpreter.handleWindowsDragDrop();");
                    }
                }
                UserWindowEvent::WindowsDragLeave(id) => {
                    if let Some(app_webview) = app.webviews.get(&id) {
                        _ = app_webview
                            .desktop_context
                            .webview
                            .evaluate_script("window.interpreter.handleWindowsDragLeave();");
                    }
                }
                UserWindowEvent::WindowsDragOver(id, x_pos, y_pos) => {
                    if let Some(app_webview) = app.webviews.get(&id) {
                        _ = app_webview
                            .desktop_context
                            .webview
                            .evaluate_script(&format!(
                                "window.interpreter.handleWindowsDragOver({x_pos}, {y_pos});"
                            ));
                    }
                }

                UserWindowEvent::Ipc { id, msg } => match msg.method() {
                    IpcMethod::Initialize => app.handle_initialize_msg(id),
                    IpcMethod::UserEvent => {}
                    IpcMethod::Query => app.handle_query_msg(msg, id),
                    IpcMethod::BrowserOpen => app.handle_browser_open(msg),
                    IpcMethod::Other(_) => {}
                },
            },
            _ => {}
        }

        *control_flow = app.control_flow;
    })
}

/// Run a desktop [`VirtualDom`] directly.
///
/// Unlike [`launch`], this raw entrypoint does not wrap the root component in a native window.
/// The root component must render at least one [`Window`](crate::Window) component manually if the
/// app should show UI.
///
/// ```rust, ignore
/// use dioxus::prelude::*;
/// use dioxus_desktop::{Config, Window};
///
/// fn app() -> Element {
///     rsx! {
///         Window {
///             div { "hello from a manually owned window" }
///         }
///     }
/// }
///
/// let dom = VirtualDom::new(app);
/// dioxus_desktop::launch::launch_virtual_dom(dom, Config::new());
/// ```
pub fn launch_virtual_dom(virtual_dom: VirtualDom, desktop_config: Config) -> ! {
    #[cfg(feature = "tokio_runtime")]
    {
        if let std::result::Result::Ok(handle) = tokio::runtime::Handle::try_current() {
            assert_ne!(
                handle.runtime_flavor(),
                tokio::runtime::RuntimeFlavor::CurrentThread,
                "The tokio current-thread runtime does not work with dioxus event handling"
            );
            launch_virtual_dom_blocking(virtual_dom, desktop_config);
        } else {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(tokio::task::unconstrained(async move {
                    launch_virtual_dom_blocking(virtual_dom, desktop_config)
                }));

            unreachable!("The desktop launch function will never exit")
        }
    }

    #[cfg(not(feature = "tokio_runtime"))]
    {
        launch_virtual_dom_blocking(virtual_dom, desktop_config);
    }
}

/// Launch a desktop app.
///
/// By default, this wraps the user component in a default [`Window`](crate::Window). Set
/// [`Config::with_headless_root`] to run the root component without an implicit window.
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
) -> ! {
    let mut config = *platform_config
        .into_iter()
        .find_map(|cfg| cfg.downcast::<Config>().ok())
        .unwrap_or_default();

    let mut virtual_dom = if config.headless_root {
        VirtualDom::new_with_props(LaunchedRoot, LaunchedRootProps { root })
    } else {
        // The app keeps the application-wide settings; the window settings move into the default
        // `Window` component we wrap the user's root in.
        let window_config = std::mem::take(&mut config.window);
        let root_props = DesktopRootProps {
            root,
            config: Rc::new(RefCell::new(Some(window_config))),
        };
        VirtualDom::new_with_props(WindowedRoot, root_props)
    };

    for context in contexts {
        virtual_dom.insert_any_root_context(context());
    }

    launch_virtual_dom(virtual_dom, config)
}
