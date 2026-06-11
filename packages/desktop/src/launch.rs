use crate::Config;
use crate::app::MakeVirtualDom;
use crate::{
    app::App,
    ipc::{IpcMethod, UserWindowEventVariant},
};
use dioxus_core::*;
use std::any::Any;
use tao::event::{Event, StartCause, WindowEvent};

/// Launch the WebView and run the event loop, with configuration and root props.
///
/// This will block the main thread, and *must* be spawned on the main thread. This function does not assume any runtime
/// and is equivalent to calling launch_with_props with the tokio feature disabled.
pub fn launch_virtual_dom_blocking(
    virtual_dom: impl FnOnce() -> VirtualDom + Send + 'static,
    mut desktop_config: Config,
) -> ! {
    let mut custom_event_handler = desktop_config.custom_event_handler.take();
    let virtual_dom = Box::new(virtual_dom);
    let (event_loop, mut app) = App::new(desktop_config, virtual_dom);

    event_loop.run(move |window_event, event_loop, control_flow| {
        // Set the control flow and check if any events need to be handled in the app itself
        let window_event = app.tick(window_event, event_loop);

        let _lock = crate::android_sync_lock::android_runtime_lock();

        if let Some(ref mut f) = custom_event_handler {
            f(&window_event, event_loop)
        }

        match window_event {
            Event::NewEvents(cause) => {
                if matches!(cause, StartCause::Init) {
                    app.handle_start_cause_init(event_loop);
                }
            }
            Event::LoopDestroyed => app.handle_loop_destroyed(),
            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => app.handle_close_requested(window_id),
                WindowEvent::Destroyed { .. } => app.begin_window_close(window_id),
                WindowEvent::Resized(new_size) => app.resize_window(window_id, new_size),
                _ => {}
            },

            Event::UserEvent(event) => match event.into_variant() {
                UserWindowEventVariant::NewWindow => app.handle_new_window(event_loop),
                UserWindowEventVariant::CloseWindow(id) => app.handle_close_requested(id),
                UserWindowEventVariant::DestroyWindow(id) => app.begin_window_close(id),
                UserWindowEventVariant::AllWindowHandlesDropped(id) => {
                    app.window_handles_dropped(id)
                }
                UserWindowEventVariant::Shutdown => app.handle_shutdown(),

                #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
                UserWindowEventVariant::GlobalHotKeyEvent(evnt) => app.handle_global_hotkey(evnt),

                #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
                UserWindowEventVariant::MudaMenuEvent(evnt) => app.handle_menu_event(evnt),

                #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
                UserWindowEventVariant::TrayMenuEvent(evnt) => app.handle_tray_menu_event(evnt),

                #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
                UserWindowEventVariant::TrayIconEvent(evnt) => app.handle_tray_icon_event(evnt),

                #[cfg(all(feature = "devtools", debug_assertions))]
                UserWindowEventVariant::HotReloadEvent(msg) => app.handle_hot_reload_msg(msg),

                // Windows-only drag-n-drop fix events.
                UserWindowEventVariant::WindowsDragDrop(id) => {
                    if let Some(webview) = app.webviews.get(&id) {
                        let _ = webview
                            .desktop_context
                            .webview
                            .evaluate_script("window.interpreter.handleWindowsDragDrop();");
                    }
                }
                UserWindowEventVariant::WindowsDragLeave(id) => {
                    if let Some(webview) = app.webviews.get(&id) {
                        let _ = webview
                            .desktop_context
                            .webview
                            .evaluate_script("window.interpreter.handleWindowsDragLeave();");
                    }
                }
                UserWindowEventVariant::WindowsDragOver(id, x_pos, y_pos) => {
                    if let Some(webview) = app.webviews.get(&id) {
                        let _ = webview.desktop_context.webview.evaluate_script(&format!(
                            "window.interpreter.handleWindowsDragOver({x_pos}, {y_pos});"
                        ));
                    }
                }

                UserWindowEventVariant::Ipc { id, msg } => match msg.method() {
                    IpcMethod::Initialize => app.handle_initialize_msg(id),
                    IpcMethod::BrowserOpen => app.handle_browser_open(msg),
                    IpcMethod::Other(_) => {}
                },

                // The edit websocket rebound to a new port; re-point every webview at it.
                UserWindowEventVariant::ReconnectEdits => {
                    app.reconnect_all_edits();
                }

                UserWindowEventVariant::WryBindgenDriverWake(id) => {
                    app.poll_wry_bindgen_driver(id);
                }

                // Run a closure with DesktopService access on the main thread
                UserWindowEventVariant::RunWithDesktopService {
                    window_id,
                    callback,
                } => {
                    // Every sender holds a strong WindowHandle and the window only leaves the
                    // map after the last handle drops (AllWindowHandlesDropped), so the lookup
                    // cannot miss. A miss is a dioxus-desktop teardown-ordering bug.
                    let webview = app.webviews.get(&window_id).expect(
                        "a window's main-thread state outlives all of its DesktopContexts",
                    );
                    callback.run(&webview.desktop_context);
                }
            },
            _ => {}
        }

        *control_flow = app.control_flow;
    })
}

/// Launches the WebView and runs the event loop, with configuration and root props.
pub fn launch_virtual_dom(
    virtual_dom: impl FnOnce() -> VirtualDom + Send + 'static,
    desktop_config: Config,
) -> ! {
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

/// Launches the WebView and runs the event loop, with configuration and root props.
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
) -> ! {
    // Create a factory function that builds the VirtualDom with contexts
    let make_dom: MakeVirtualDom = Box::new(move || {
        let mut virtual_dom = VirtualDom::new(root);

        for context in contexts {
            virtual_dom.insert_any_root_context(context());
        }

        virtual_dom
    });

    let platform_config = *platform_config
        .into_iter()
        .find_map(|cfg| cfg.downcast::<Config>().ok())
        .unwrap_or_default();
    launch_virtual_dom(make_dom, platform_config)
}
