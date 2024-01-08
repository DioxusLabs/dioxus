use crate::{
    assets::*, ipc::UserWindowEvent, shortcut::IntoAccelerator, window, DesktopContext,
    ShortcutHandle, ShortcutRegistryError, WryEventHandler,
};
use dioxus_core::ScopeState;
use tao::{event::Event, event_loop::EventLoopWindowTarget};
use wry::RequestAsyncResponder;

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
    cx: &ScopeState,
    name: &str,
    handler: impl Fn(AssetRequest, RequestAsyncResponder) + 'static,
) {
    cx.use_hook(|| {
        crate::window().asset_handlers.register_handler(
            name.to_string(),
            Box::new(handler),
            cx.scope_id(),
        );

        Handler(name.to_string())
    });

    // todo: can we just put ondrop in core?
    struct Handler(String);
    impl Drop for Handler {
        fn drop(&mut self) {
            _ = crate::window().asset_handlers.remove_handler(&self.0);
        }
    }
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
