// use crate::Config;
// use crate::{
//     app::App,
//     ipc::{IpcMethod, UserWindowEvent},
// };
// use dioxus_core::*;
// use dioxus_document::eval;
// use std::any::Any;
// use tao::event::{Event, StartCause, WindowEvent};

// /// Launches the WebView and runs the event loop
// pub fn launch(root: fn() -> Element) {
//     launch_cfg(root, vec![], vec![]);
// }

// /// Launches the WebView and runs the event loop, with configuration and root props.
// pub fn launch_cfg(
//     root: fn() -> Element,
//     contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
//     platform_config: Vec<Box<dyn Any>>,
// ) {
//     let mut virtual_dom = VirtualDom::new(root);

//     for context in contexts {
//         virtual_dom.insert_any_root_context(context());
//     }

//     let platform_config = *platform_config
//         .into_iter()
//         .find_map(|cfg| cfg.downcast::<Config>().ok())
//         .unwrap_or_default();

//     launch_virtual_dom(virtual_dom, platform_config)
// }

// /// Launches the WebView and runs the event loop, with configuration and root props.
// pub fn launch_virtual_dom(virtual_dom: VirtualDom, desktop_config: Config) -> ! {
//     #[cfg(feature = "tokio_runtime")]
//     {
//         tokio::runtime::Builder::new_multi_thread()
//             .enable_all()
//             .build()
//             .unwrap()
//             .block_on(tokio::task::unconstrained(async move {
//                 launch_virtual_dom_blocking(virtual_dom, desktop_config)
//             }));

//         unreachable!("The desktop launch function will never exit")
//     }

//     #[cfg(not(feature = "tokio_runtime"))]
//     {
//         launch_virtual_dom_blocking(virtual_dom, desktop_config);
//     }
// }

// /// Launch the WebView and run the event loop, with configuration and root props.
// ///
// /// This will block the main thread, and *must* be spawned on the main thread. This function does not assume any runtime
// /// and is equivalent to calling launch_with_props with the tokio feature disabled.
// pub fn launch_virtual_dom_blocking(virtual_dom: VirtualDom, mut desktop_config: Config) -> ! {
//     let mut custom_event_handler = desktop_config.custom_event_handler.take();
//     let (event_loop, mut app) = App::new(desktop_config, virtual_dom);

//     event_loop.run(move |window_event, event_loop, control_flow| {
//         // Set the control flow and check if any events need to be handled in the app itself
//         app.tick(&window_event);

//         if let Some(ref mut f) = custom_event_handler {
//             f(&window_event, event_loop)
//         }

//         match window_event {
//             Event::NewEvents(StartCause::Init) => app.handle_start_cause_init(),
//             Event::LoopDestroyed => app.handle_loop_destroyed(),
//             Event::WindowEvent {
//                 event, window_id, ..
//             } => match event {
//                 WindowEvent::CloseRequested => app.handle_close_requested(window_id),
//                 WindowEvent::Destroyed { .. } => app.window_destroyed(window_id),
//                 WindowEvent::Resized(new_size) => app.resize_window(window_id, new_size),
//                 _ => {}
//             },

//             Event::UserEvent(event) => match event {
//                 UserWindowEvent::Poll(id) => app.poll_vdom(id),
//                 UserWindowEvent::NewWindow => app.handle_new_window(),
//                 UserWindowEvent::CloseWindow(id) => app.handle_close_msg(id),
//                 UserWindowEvent::Shutdown => app.control_flow = tao::event_loop::ControlFlow::Exit,

//                 #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
//                 UserWindowEvent::GlobalHotKeyEvent(evnt) => app.handle_global_hotkey(evnt),

//                 #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
//                 UserWindowEvent::MudaMenuEvent(evnt) => app.handle_menu_event(evnt),

//                 #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
//                 UserWindowEvent::TrayMenuEvent(evnt) => app.handle_tray_menu_event(evnt),

//                 #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
//                 UserWindowEvent::TrayIconEvent(evnt) => app.handle_tray_icon_event(evnt),

//                 #[cfg(all(feature = "devtools", debug_assertions))]
//                 UserWindowEvent::HotReloadEvent(msg) => app.handle_hot_reload_msg(msg),

//                 // Windows-only drag-n-drop fix events. We need to call the interpreter drag-n-drop code.
//                 UserWindowEvent::WindowsDragDrop(id) => {
//                     if let Some(webview) = app.webviews.get(&id) {
//                         webview.dom.in_runtime(|| {
//                             ScopeId::ROOT.in_runtime(|| {
//                                 eval("window.interpreter.handleWindowsDragDrop();");
//                             });
//                         });
//                     }
//                 }
//                 UserWindowEvent::WindowsDragLeave(id) => {
//                     if let Some(webview) = app.webviews.get(&id) {
//                         webview.dom.in_runtime(|| {
//                             ScopeId::ROOT.in_runtime(|| {
//                                 eval("window.interpreter.handleWindowsDragLeave();");
//                             });
//                         });
//                     }
//                 }
//                 UserWindowEvent::WindowsDragOver(id, x_pos, y_pos) => {
//                     if let Some(webview) = app.webviews.get(&id) {
//                         webview.dom.in_runtime(|| {
//                             ScopeId::ROOT.in_runtime(|| {
//                                 let e = eval(
//                                     r#"
//                                     const xPos = await dioxus.recv();
//                                     const yPos = await dioxus.recv();
//                                     window.interpreter.handleWindowsDragOver(xPos, yPos)
//                                     "#,
//                                 );

//                                 _ = e.send(x_pos);
//                                 _ = e.send(y_pos);
//                             });
//                         });
//                     }
//                 }

//                 UserWindowEvent::Ipc { id, msg } => match msg.method() {
//                     IpcMethod::Initialize => app.handle_initialize_msg(id),
//                     IpcMethod::FileDialog => app.handle_file_dialog_msg(msg, id),
//                     IpcMethod::UserEvent => {}
//                     IpcMethod::Query => app.handle_query_msg(msg, id),
//                     IpcMethod::BrowserOpen => app.handle_browser_open(msg),
//                     IpcMethod::Other(_) => {}
//                 },
//             },
//             _ => {}
//         }

//         *control_flow = app.control_flow;
//     })
// }

// /// Expose the `Java_dev_dioxus_main_WryActivity_create` function to the JNI layer.
// /// We hardcode these to have a single trampoline for host Java code to call into.
// ///
// /// This saves us from having to plumb the top-level package name all the way down into
// /// this file. This is better for modularity (ie just call dioxus' main to run the app) as
// /// well as cache thrashing since this crate doesn't rely on external env vars.
// ///
// /// The CLI is expecting to find `dev.dioxus.main` in the final library. If you find a need to
// /// change this, you'll need to change the CLI as well.
// #[cfg(target_os = "android")]
// #[no_mangle]
// #[inline(never)]
// pub extern "C" fn start_app() {
//     wry::android_binding!(dev_dioxus, main, wry);
//     tao::android_binding!(
//         dev_dioxus,
//         main,
//         WryActivity,
//         wry::android_setup,
//         dioxus_main_root_fn,
//         tao
//     );
// }

// #[cfg(target_os = "android")]
// #[doc(hidden)]
// pub fn dioxus_main_root_fn() {
//     // we're going to find the `main` symbol using dlsym directly and call it
//     unsafe {
//         let mut main_fn_ptr = libc::dlsym(libc::RTLD_DEFAULT, b"main\0".as_ptr() as _);

//         if main_fn_ptr.is_null() {
//             main_fn_ptr = libc::dlsym(libc::RTLD_DEFAULT, b"_main\0".as_ptr() as _);
//         }

//         if main_fn_ptr.is_null() {
//             panic!("Failed to find main symbol");
//         }

//         // Set the env vars that rust code might expect, passed off to us by the android app
//         // Doing this before main emulates the behavior of a regular executable
//         if cfg!(target_os = "android") && cfg!(debug_assertions) {
//             load_env_file_from_session_cache();
//         }

//         let main_fn: extern "C" fn() = std::mem::transmute(main_fn_ptr);
//         main_fn();
//     };
// }

// /// Load the env file from the session cache if we're in debug mode and on android
// ///
// /// This is a slightly hacky way of being able to use std::env::var code in android apps without
// /// going through their custom java-based system.
// #[cfg(target_os = "android")]
// fn load_env_file_from_session_cache() {
//     let env_file = dioxus_cli_config::android_session_cache_dir().join(".env");
//     if let Some(env_file) = std::fs::read_to_string(&env_file).ok() {
//         for line in env_file.lines() {
//             if let Some((key, value)) = line.trim().split_once('=') {
//                 std::env::set_var(key, value);
//             }
//         }
//     }
// }

use crate::Config;
use crate::{
    app::App,
    ipc::{IpcMethod, UserWindowEvent},
};
use dioxus_core::*;
use dioxus_document::eval;
use std::any::Any;
use tao::event::{Event, StartCause, WindowEvent};

/// Launch the WebView and run the event loop, with configuration and root props.
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
                UserWindowEvent::Poll(id) => app.poll_vdom(id),
                UserWindowEvent::NewWindow => app.handle_new_window(),
                UserWindowEvent::CloseWindow(id) => app.handle_close_msg(id),
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
                    if let Some(webview) = app.webviews.get(&id) {
                        webview.dom.in_runtime(|| {
                            ScopeId::ROOT.in_runtime(|| {
                                eval("window.interpreter.handleWindowsDragDrop();");
                            });
                        });
                    }
                }
                UserWindowEvent::WindowsDragLeave(id) => {
                    if let Some(webview) = app.webviews.get(&id) {
                        webview.dom.in_runtime(|| {
                            ScopeId::ROOT.in_runtime(|| {
                                eval("window.interpreter.handleWindowsDragLeave();");
                            });
                        });
                    }
                }
                UserWindowEvent::WindowsDragOver(id, x_pos, y_pos) => {
                    if let Some(webview) = app.webviews.get(&id) {
                        webview.dom.in_runtime(|| {
                            ScopeId::ROOT.in_runtime(|| {
                                let e = eval(
                                    r#"
                                    const xPos = await dioxus.recv();
                                    const yPos = await dioxus.recv();
                                    window.interpreter.handleWindowsDragOver(xPos, yPos)
                                    "#,
                                );

                                _ = e.send(x_pos);
                                _ = e.send(y_pos);
                            });
                        });
                    }
                }

                UserWindowEvent::Ipc { id, msg } => match msg.method() {
                    IpcMethod::Initialize => app.handle_initialize_msg(id),
                    IpcMethod::FileDialog => app.handle_file_dialog_msg(msg, id),
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
