use std::rc::Rc;

use crate::{assets::*, shortcut::IntoAccelerator, ShortcutHandle, ShortcutRegistryError};
use crate::{desktop_context::UserWindowEvent, window, DesktopContext, WryEventHandler};
use dioxus_core::ScopeState;
use tao::{event::Event, event_loop::EventLoopWindowTarget};

/// Get an imperative handle to the current window
pub fn use_window(cx: &ScopeState) -> &DesktopContext {
    cx.use_hook(|| cx.consume_context::<DesktopContext>())
        .as_ref()
        .unwrap()
}

/// Get a closure that executes any JavaScript in the WebView context.
pub fn use_wry_event_handler(
    cx: &ScopeState,
    handler: impl FnMut(&Event<UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>) + 'static,
) -> &WryEventHandler {
    cx.use_hook(move || {
        let desktop = window();

        let id = desktop.create_wry_event_handler(handler);

        WryEventHandler {
            handlers: desktop.event_handlers.clone(),
            id,
        }
    })
}

/// Provide a callback to handle asset loading yourself.
///
/// The callback takes a path as requested by the web view, and it should return `Some(response)`
/// if you want to load the asset, and `None` if you want to fallback on the default behavior.
pub fn use_asset_handler<F: AssetFuture>(
    cx: &ScopeState,
    handler: impl AssetHandler<F>,
) -> &AssetHandlerHandle {
    cx.use_hook(|| {
        let desktop = crate::window();
        let handler_id = Rc::new(tokio::sync::OnceCell::new());
        let handler_id_ref = Rc::clone(&handler_id);
        let desktop_ref = Rc::clone(&desktop);
        cx.push_future(async move {
            let id = desktop.asset_handlers.register_handler(handler).await;
            handler_id.set(id).unwrap();
        });
        AssetHandlerHandle {
            desktop: desktop_ref,
            handler_id: handler_id_ref,
        }
    })
}

/// Get a closure that executes any JavaScript in the WebView context.
pub fn use_global_shortcut(
    cx: &ScopeState,
    accelerator: impl IntoAccelerator,
    handler: impl FnMut() + 'static,
) -> &Result<ShortcutHandle, ShortcutRegistryError> {
    cx.use_hook(move || {
        let desktop = window();

        let id = desktop.create_shortcut(accelerator.accelerator(), handler);

        Ok(ShortcutHandle {
            desktop,
            shortcut_id: id?,
        })
    })
}
