use std::rc::Rc;

use crate::{
    assets::*, ipc::UserWindowEvent, shortcut::IntoAccelerator, window, DesktopContext,
    ShortcutHandle, ShortcutRegistryError, WryEventHandler,
};
use dioxus_core::{
    prelude::{consume_context, current_scope_id},
    use_hook,
};
use dioxus_hooks::use_on_drop;
use tao::{event::Event, event_loop::EventLoopWindowTarget};
use wry::RequestAsyncResponder;

/// Get an imperative handle to the current window
pub fn use_window() -> DesktopContext {
    use_hook(|| consume_context::<DesktopContext>())
}

/// Get a closure that executes any JavaScript in the WebView context.
pub fn use_wry_event_handler(
    handler: impl FnMut(&Event<UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>) + 'static,
) -> WryEventHandler {
    use_hook(move || {
        let desktop = window();

        let id = desktop.create_wry_event_handler(handler);

        WryEventHandler {
            handlers: desktop.shared.event_handlers.clone(),
            id,
        }
    })
}

/// Provide a callback to handle asset loading yourself.
///
/// The callback takes a path as requested by the web view, and it should return `Some(response)`
/// if you want to load the asset, and `None` if you want to fallback on the default behavior.
pub fn use_asset_handler(
    name: &str,
    handler: impl Fn(AssetRequest, RequestAsyncResponder) + 'static,
) {
    let name = use_hook(|| {
        crate::window().asset_handlers.register_handler(
            name.to_string(),
            Box::new(handler),
            current_scope_id().unwrap(),
        );

        Rc::new(name.to_string())
    });

    use_on_drop(move || {
        _ = crate::window().asset_handlers.remove_handler(name.as_ref());
    });
}

/// Get a closure that executes any JavaScript in the WebView context.
pub fn use_global_shortcut(
    accelerator: impl IntoAccelerator,
    handler: impl FnMut() + 'static,
) -> Result<ShortcutHandle, ShortcutRegistryError> {
    use_hook(move || {
        let desktop = window();

        let id = desktop.create_shortcut(accelerator.accelerator(), handler);

        Ok(ShortcutHandle {
            desktop,
            shortcut_id: id?,
        })
    })
}
