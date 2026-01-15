use std::rc::Rc;

use crate::{
    assets::*, ipc::UserWindowEvent, shortcut::IntoAccelerator, window, DesktopContext,
    HotKeyState, ShortcutHandle, ShortcutRegistryError, WryEventHandler,
};
use dioxus_core::{consume_context, use_hook, use_hook_with_cleanup, Runtime};

use dioxus_hooks::use_callback;
use tao::{event::Event, event_loop::EventLoopWindowTarget};
use wry::RequestAsyncResponder;

/// Get an imperative handle to the current window
pub fn use_window() -> DesktopContext {
    use_hook(consume_context::<DesktopContext>)
}

/// Register an event handler that runs when a wry event is processed.
pub fn use_wry_event_handler(
    mut handler: impl FnMut(&Event<UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>) + 'static,
) -> WryEventHandler {
    use dioxus_core::current_scope_id;

    // Capture the current runtime and scope ID.
    let runtime = Runtime::current();
    let scope_id = current_scope_id();

    use_hook_with_cleanup(
        move || {
            window().create_wry_event_handler(move |event, target| {
                runtime.in_scope(scope_id, || handler(event, target))
            })
        },
        move |handler| handler.remove(),
    )
}

/// Register an event handler that runs when a muda event is processed.
#[cfg_attr(
    docsrs,
    doc(cfg(any(target_os = "windows", target_os = "linux", target_os = "macos")))
)]
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub fn use_muda_event_handler(
    mut handler: impl FnMut(&muda::MenuEvent) + 'static,
) -> WryEventHandler {
    use_wry_event_handler(move |event, _| {
        if let Event::UserEvent(UserWindowEvent::MudaMenuEvent(event)) = event {
            handler(event);
        }
    })
}

/// Register an event handler that runs when a tray icon menu event is processed.
#[cfg_attr(
    docsrs,
    doc(cfg(any(target_os = "windows", target_os = "linux", target_os = "macos")))
)]
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub fn use_tray_menu_event_handler(
    mut handler: impl FnMut(&tray_icon::menu::MenuEvent) + 'static,
) -> WryEventHandler {
    use_wry_event_handler(move |event, _| {
        if let Event::UserEvent(UserWindowEvent::TrayMenuEvent(event)) = event {
            handler(event);
        }
    })
}

/// Register an event handler that runs when a tray icon event is processed.
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
    use_wry_event_handler(move |event, _| {
        if let Event::UserEvent(UserWindowEvent::TrayIconEvent(event)) = event {
            handler(event);
        }
    })
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
            crate::window()
                .asset_handlers
                .register_handler(name.to_string(), cb);

            Rc::new(name.to_string())
        },
        move |name| {
            _ = crate::window().asset_handlers.remove_handler(name.as_ref());
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
