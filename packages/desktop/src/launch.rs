use crate::Config;
use crate::{app::App, ipc::UserWindowEvent};
use dioxus_core::*;
use std::any::Any;
use winit::application::ApplicationHandler;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

/// Launch the WebView and run the event loop, with configuration and root props.
///
/// This will block the main thread, and *must* be spawned on the main thread. This function does not assume any runtime
/// and is equivalent to calling launch_with_props with the tokio feature disabled.
pub fn launch_virtual_dom_blocking(virtual_dom: VirtualDom, desktop_config: Config) -> ! {
    let event_loop = EventLoop::<UserWindowEvent>::with_user_event()
        .build()
        .unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();

    let mut app = App::new(proxy, desktop_config, virtual_dom);
    event_loop.run_app(&mut app);

    std::process::exit(0);

    // event_loop.run(move |window_event, event_loop, control_flow| {
    //     // Set the control flow and check if any events need to be handled in the app itself
    //     app.tick(&window_event);

    //     if let Some(ref mut f) = custom_event_handler {
    //         f(&window_event, event_loop)
    //     }

    //     match window_event {
    //         Event::NewEvents(StartCause::Init) => app.handle_start_cause_init(),
    //         Event::LoopDestroyed => app.handle_loop_destroyed(),
    //         Event::WindowEvent {
    //             event, window_id, ..
    //         } => match event {
    //             WindowEvent::CloseRequested => app.handle_close_requested(window_id),
    //             WindowEvent::Destroyed { .. } => app.window_destroyed(window_id),
    //             WindowEvent::Resized(new_size) => app.resize_window(window_id, new_size),
    //             _ => {}
    //         },

    //         Event::UserEvent(event) => match event {
    //             UserWindowEvent::Poll(id) => app.poll_vdom(id),
    //             UserWindowEvent::NewWindow => {
    //                 // Create new windows/webviews
    //                 {
    //                     let mut pending_windows = app.shared.pending_windows.borrow_mut();
    //                     let mut pending_webviews = app.shared.pending_webviews.borrow_mut();

    //                     for (dom, cfg, sender) in pending_windows.drain(..) {
    //                         let window = WebviewInstance::new(cfg, dom, app.shared.clone());

    //                         // Send the desktop context to the MaybeDesktopService
    //                         let cx = window.dom.in_runtime(|| {
    //                             ScopeId::ROOT
    //                                 .consume_context::<Rc<DesktopService>>()
    //                                 .unwrap()
    //                         });
    //                         let _ = sender.send(Rc::downgrade(&cx));

    //                         pending_webviews.push(window);
    //                     }
    //                 }

    //                 app.handle_new_windows();
    //             }
    //             UserWindowEvent::CloseWindow(id) => app.handle_close_msg(id),
    //             UserWindowEvent::Shutdown => app.control_flow = tao::event_loop::ControlFlow::Exit,

    //             #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    //             UserWindowEvent::GlobalHotKeyEvent(evnt) => app.handle_global_hotkey(evnt),

    //             #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    //             UserWindowEvent::MudaMenuEvent(evnt) => app.handle_menu_event(evnt),

    //             #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    //             UserWindowEvent::TrayMenuEvent(evnt) => app.handle_tray_menu_event(evnt),

    //             #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    //             UserWindowEvent::TrayIconEvent(evnt) => app.handle_tray_icon_event(evnt),

    //             #[cfg(all(feature = "devtools", debug_assertions))]
    //             UserWindowEvent::HotReloadEvent(msg) => app.handle_hot_reload_msg(msg),

    //             // Windows-only drag-n-drop fix events. We need to call the interpreter drag-n-drop code.
    //             UserWindowEvent::WindowsDragDrop(id) => {
    //                 if let Some(webview) = app.webviews.get(&id) {
    //                     webview.dom.in_runtime(|| {
    //                         ScopeId::ROOT.in_runtime(|| {
    //                             eval("window.interpreter.handleWindowsDragDrop();");
    //                         });
    //                     });
    //                 }
    //             }
    //             UserWindowEvent::WindowsDragLeave(id) => {
    //                 if let Some(webview) = app.webviews.get(&id) {
    //                     webview.dom.in_runtime(|| {
    //                         ScopeId::ROOT.in_runtime(|| {
    //                             eval("window.interpreter.handleWindowsDragLeave();");
    //                         });
    //                     });
    //                 }
    //             }
    //             UserWindowEvent::WindowsDragOver(id, x_pos, y_pos) => {
    //                 if let Some(webview) = app.webviews.get(&id) {
    //                     webview.dom.in_runtime(|| {
    //                         ScopeId::ROOT.in_runtime(|| {
    //                             let e = eval(
    //                                 r#"
    //                                 const xPos = await dioxus.recv();
    //                                 const yPos = await dioxus.recv();
    //                                 window.interpreter.handleWindowsDragOver(xPos, yPos)
    //                                 "#,
    //                             );

    //                             _ = e.send(x_pos);
    //                             _ = e.send(y_pos);
    //                         });
    //                     });
    //                 }
    //             }

    //             UserWindowEvent::Ipc { id, msg } => match msg.method() {
    //                 IpcMethod::Initialize => app.handle_initialize_msg(id),
    //                 IpcMethod::FileDialog => app.handle_file_dialog_msg(msg, id),
    //                 IpcMethod::UserEvent => {}
    //                 IpcMethod::Query => app.handle_query_msg(msg, id),
    //                 IpcMethod::BrowserOpen => app.handle_browser_open(msg),
    //                 IpcMethod::Other(_) => {}
    //             },
    //         },
    //         _ => {}
    //     }

    //     *control_flow = app.control_flow;
    // })
}

impl ApplicationHandler<UserWindowEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        todo!()
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.handle_event(event_loop, &Event::WindowEvent { window_id, event });
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserWindowEvent) {
        self.handle_event(event_loop, &Event::UserEvent(event));
    }
}

/// Launches the WebView and runs the event loop, with configuration and root props.
pub fn launch_virtual_dom(virtual_dom: VirtualDom, desktop_config: Config) -> ! {
    #[cfg(feature = "tokio_runtime")]
    {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(tokio::task::unconstrained(async move {
                launch_virtual_dom_blocking(virtual_dom, desktop_config)
            }));

        unreachable!("The desktop launch function will never exit")
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
    let mut virtual_dom = VirtualDom::new(root);

    for context in contexts {
        virtual_dom.insert_any_root_context(context());
    }

    let platform_config = *platform_config
        .into_iter()
        .find_map(|cfg| cfg.downcast::<Config>().ok())
        .unwrap_or_default();

    launch_virtual_dom(virtual_dom, platform_config)
}
