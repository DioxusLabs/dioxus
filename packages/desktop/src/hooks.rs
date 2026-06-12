use std::rc::Rc;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
use crate::dom_thread::VirtualDomEvent;
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
use crate::ipc::UserWindowEventVariant;
use crate::{
    DesktopContext, HotKeyState, ShortcutHandle, ShortcutRegistryError, WryEventHandler, assets::*,
    ipc::UserWindowEvent, shortcut::IntoAccelerator, window,
};
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
use dioxus_core::{Runtime, current_scope_id};
use dioxus_core::{consume_context, use_hook, use_hook_with_cleanup};

use dioxus_hooks::use_callback;
use tao::{event::Event, event_loop::EventLoopWindowTarget};
use wry::RequestAsyncResponder;

/// Get an imperative handle to the current window
pub fn use_window() -> DesktopContext {
    use_hook(consume_context::<DesktopContext>)
}

/// Register an event handler that runs on the VirtualDom thread when a wry event is processed.
///
/// The handler stays on the VirtualDom thread (like [`use_asset_handler`]), so it does **not**
/// need to be `Send`: it can capture signals and call blocking [`DesktopContext`] APIs such as
/// [`set_title`](DesktopContext::set_title). Events are cloned and queued over to the
/// VirtualDom thread, so they arrive asynchronously, after the event loop has already finished
/// processing them.
///
/// The one event that never reaches this handler is
/// [`WindowEvent::ScaleFactorChanged`](tao::event::WindowEvent::ScaleFactorChanged): it borrows
/// from the event loop and cannot be sent across threads. Tao delivers a
/// [`WindowEvent::Resized`](tao::event::WindowEvent::Resized) alongside it, so size changes are
/// still observed; query [`scale_factor`](tao::window::Window::scale_factor) if you need the new
/// scale. If you need that event, synchronous delivery, or the [`EventLoopWindowTarget`], use
/// [`use_main_thread_wry_event_handler`] instead.
///
/// ```rust, ignore
/// use_wry_event_handler(move |event| {
///     if let Event::WindowEvent { event: WindowEvent::Focused(focused), .. } = event {
///         // Signals are fine here: this closure never leaves the VirtualDom thread.
///     }
/// });
/// ```
///
/// The handler is removed automatically when the component is dropped.
#[cfg_attr(
    docsrs,
    doc(cfg(any(target_os = "windows", target_os = "linux", target_os = "macos")))
)]
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub fn use_wry_event_handler(handler: impl FnMut(&Event<()>) + 'static) -> WryEventHandler {
    use_hook_with_cleanup(
        move || window().create_wry_event_handler(handler),
        move |handler| handler.remove(),
    )
}

/// Register an event handler that runs synchronously on the main event loop thread for every wry
/// event, with access to the [`EventLoopWindowTarget`].
///
/// Unlike [`use_wry_event_handler`], this sees every event — including
/// [`WindowEvent::ScaleFactorChanged`](tao::event::WindowEvent::ScaleFactorChanged) — before the
/// event loop continues. Because the closure is moved to the main thread, it must be `Send`.
///
/// Capturing a [`DesktopContext`] is rejected because its blocking APIs would deadlock the event
/// loop waiting on itself (and the context is not `Send`). Use [`use_wry_event_handler`] if you
/// need to update signals or call [`DesktopContext`] APIs.
///
/// ```rust, compile_fail
/// use dioxus_desktop::{use_main_thread_wry_event_handler, use_window};
///
/// fn app() {
///     let desktop = use_window();
///     use_main_thread_wry_event_handler(move |_event, _target| {
///         desktop.set_title("will not compile");
///     });
/// }
/// ```
///
/// The handler is removed automatically when the component is dropped.
pub fn use_main_thread_wry_event_handler(
    handler: impl FnMut(&Event<()>, &EventLoopWindowTarget<UserWindowEvent>) + Send + 'static,
) -> WryEventHandler {
    use_hook_with_cleanup(
        move || window().create_main_thread_wry_event_handler(handler),
        move |handler| handler.remove(),
    )
}

/// Register a handler on the VirtualDom thread for main-thread events selected by
/// `forward_event`. The selected payload is sent across threads (hence `T: Send`), while
/// `handler` stays on the VirtualDom thread and runs in the calling component's scope.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
fn use_dom_event_handler<T: Send + 'static>(
    mut handler: impl FnMut(T) + 'static,
    mut forward_event: impl FnMut(&UserWindowEvent) -> Option<T> + Send + 'static,
) -> WryEventHandler {
    let runtime = Runtime::current();
    let scope_id = current_scope_id();
    use_hook_with_cleanup(
        move || {
            let window = window();
            let dom_handler =
                window
                    .callback_registry()
                    .register(window.window_id(), move |event: T| {
                        runtime.in_scope(scope_id, || handler(event));
                    });
            let dom_tx = window.dom_event_sender();
            let handler = window.create_wry_event_handler_with_user_event(move |event, _| {
                let Event::UserEvent(event) = event else {
                    return;
                };
                let Some(event) = forward_event(event) else {
                    return;
                };
                let _ = dom_tx.unbounded_send(VirtualDomEvent::RunCallback(Box::new(
                    move |registry| {
                        registry.invoke(dom_handler, event);
                    },
                )));
            });
            window
                .callback_registry()
                .register_wry_event_handler(handler, dom_handler);
            handler
        },
        |handler| handler.remove(),
    )
}

/// Register an event handler that runs when a muda event is processed. Unlike most callback hooks, this
/// will run outside of the virtual dom context
#[cfg_attr(
    docsrs,
    doc(cfg(any(target_os = "windows", target_os = "linux", target_os = "macos")))
)]
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub fn use_muda_event_handler(
    mut handler: impl FnMut(&muda::MenuEvent) + 'static,
) -> WryEventHandler {
    use_dom_event_handler(
        move |event| handler(&event),
        |event| match event.variant() {
            UserWindowEventVariant::MudaMenuEvent(event) => Some(event.clone()),
            _ => None,
        },
    )
}

/// Register an event handler that runs when a tray icon menu event is processed. Unlike most callback hooks, this
/// will run outside of the virtual dom context
#[cfg_attr(
    docsrs,
    doc(cfg(any(target_os = "windows", target_os = "linux", target_os = "macos")))
)]
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub fn use_tray_menu_event_handler(
    mut handler: impl FnMut(&tray_icon::menu::MenuEvent) + 'static,
) -> WryEventHandler {
    use_dom_event_handler(
        move |event| handler(&event),
        |event| match event.variant() {
            UserWindowEventVariant::TrayMenuEvent(event) => Some(event.clone()),
            _ => None,
        },
    )
}

/// Register an event handler that runs when a tray icon event is processed. Unlike most callback hooks, this
/// will run outside of the virtual dom context
/// This is only for tray icon and not it's menus.
/// If you want to register tray icon menus handler use `use_tray_menu_event_handler` instead.
#[cfg_attr(
    docsrs,
    doc(cfg(any(target_os = "windows", target_os = "linux", target_os = "macos")))
)]
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub fn use_tray_icon_event_handler(
    mut handler: impl FnMut(&tray_icon::TrayIconEvent) + 'static,
) -> WryEventHandler {
    use_dom_event_handler(
        move |event| handler(&event),
        |event| match event.variant() {
            UserWindowEventVariant::TrayIconEvent(event) => Some(event.clone()),
            _ => None,
        },
    )
}

/// Provide a callback to handle asset loading yourself.
///
/// The callback takes a path as requested by the web view, and it should return `Some(response)`
/// if you want to load the asset, and `None` if you want to fallback on the default behavior.
pub fn use_asset_handler(
    name: &str,
    mut handler: impl FnMut(AssetRequest, RequestAsyncResponder) + 'static,
) {
    // wrap the user's handler in something that keeps it up to date
    let cb = use_callback(move |(asset, responder)| handler(asset, responder));

    use_hook_with_cleanup(
        || {
            crate::window().register_asset_handler(name, move |req, resp| cb((req, resp)));

            Rc::new(name.to_string())
        },
        move |name| {
            crate::window().remove_asset_handler(name.as_ref());
        },
    );
}

/// Get a closure that executes any JavaScript in the WebView context.
pub fn use_global_shortcut(
    accelerator: impl IntoAccelerator,
    handler: impl FnMut(HotKeyState) + 'static,
) -> Result<ShortcutHandle, ShortcutRegistryError> {
    // wrap the user's handler in something that keeps it up to date
    let cb = use_callback(handler);

    use_hook_with_cleanup(
        #[allow(clippy::redundant_closure)]
        move || window().create_shortcut(accelerator.accelerator(), move |state| cb(state)),
        |handle| {
            if let Ok(handle) = handle {
                handle.remove();
            }
        },
    )
}
