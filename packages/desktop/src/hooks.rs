use std::rc::Rc;

use crate::{
    assets::*, ipc::UserWindowEvent, shortcut::IntoAccelerator, window, DesktopContext,
    ShortcutHandle, ShortcutRegistryError, WryEventHandler,
};
use dioxus_core::{
    prelude::{consume_context, use_hook_with_cleanup, RuntimeGuard},
    use_hook, Runtime,
};

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
    // move the runtime into the event handler closure
    let runtime = Runtime::current().unwrap();

    use_hook_with_cleanup(
        move || {
            window().create_wry_event_handler(move |event, target| {
                let _runtime_guard = RuntimeGuard::new(runtime.clone());
                handler(event, target)
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
    // move the runtime into the event handler closure
    let runtime = Runtime::current().unwrap();

    use_wry_event_handler(move |event, _| {
        let _runtime_guard = dioxus_core::prelude::RuntimeGuard::new(runtime.clone());
        if let Event::UserEvent(UserWindowEvent::MudaMenuEvent(event)) = event {
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
    let callback = use_callback(move |args: (AssetRequest, RequestAsyncResponder)| {
        handler(args.0, args.1);
    });

    use_hook_with_cleanup(
        || {
            crate::window()
                .asset_handlers
                .register_handler(name.to_string(), callback);

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
    mut handler: impl FnMut() + 'static,
) -> Result<ShortcutHandle, ShortcutRegistryError> {
    // wrap the user's handler in something that keeps it up to date
    let cb = use_callback(move |_| handler());

    use_hook_with_cleanup(
        move || window().create_shortcut(accelerator.accelerator(), move || cb(())),
        |handle| {
            if let Ok(handle) = handle {
                handle.remove();
            }
        },
    )
}
