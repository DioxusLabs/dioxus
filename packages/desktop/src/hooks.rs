use std::rc::Rc;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
use crate::dom_thread::VirtualDomEvent;
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
use crate::ipc::UserWindowEventVariant;
use crate::{
    DesktopContext, HotKeyState, ShortcutHandle, ShortcutRegistryError, WryEventHandler, assets::*,
    ipc::UserWindowEvent, shortcut::IntoAccelerator, window,
};
use dioxus_core::{Runtime, consume_context, current_scope_id, use_hook, use_hook_with_cleanup};

use dioxus_hooks::use_callback;
use tao::{event::Event, event_loop::EventLoopWindowTarget};
use wry::RequestAsyncResponder;

/// Get an imperative handle to the current window
pub fn use_window() -> DesktopContext {
    use_hook(consume_context::<DesktopContext>)
}

/// Marker for the [`IntoWryEventHandler`] impl whose closure also takes the event loop target.
#[doc(hidden)]
pub struct WithTargetMarker;

/// Marker for the [`IntoWryEventHandler`] impl whose closure takes only the event.
#[doc(hidden)]
pub struct WithoutTargetMarker;

/// Lets [`use_wry_event_handler`] accept either closure shape, requiring `Send` only for the
/// target-taking form.
///
/// - `FnMut(&Event<()>)` — stays on the VirtualDom thread (like [`use_asset_handler`]),
///   does **not** need to be `Send`, and cannot access the [`EventLoopWindowTarget`]. Tao events
///   that can be safely owned are queued to this handler asynchronously; events that borrow from
///   the event loop ([`WindowEvent::ScaleFactorChanged`](tao::event::WindowEvent)) are never
///   forwarded to it. This form is only available on Windows, Linux, and macOS.
/// - `FnMut(&Event<()>, &EventLoopWindowTarget<UserWindowEvent>)` — runs on the main
///   event loop thread with access to the target, and therefore must be `Send`.
///
/// The `Marker` type parameter disambiguates the two blanket impls (the technique dioxus uses for
/// `SuperInto`/`SpawnIfAsync`). Because the closure shape drives which impl applies, the closure's
/// parameters must be annotated enough for the compiler to pick the impl: write `|event: &Event<_>|`
/// for the no-target form, or `|event: &Event<_>, target: &_|` for the target form (the marker
/// indirection prevents inferring bare `|event|` / `|event, target|`).
pub trait IntoWryEventHandler<Marker> {
    /// Register the handler with the current window, returning a handle that removes it.
    fn into_wry_event_handler(self) -> WryEventHandler;
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
impl<F> IntoWryEventHandler<WithoutTargetMarker> for F
where
    F: FnMut(&Event<()>) + 'static,
{
    fn into_wry_event_handler(self) -> WryEventHandler {
        window().create_wry_event_handler_forwarding(self)
    }
}

impl<F> IntoWryEventHandler<WithTargetMarker> for F
where
    F: FnMut(&Event<()>, &EventLoopWindowTarget<UserWindowEvent>) + Send + 'static,
{
    fn into_wry_event_handler(self) -> WryEventHandler {
        window().create_wry_event_handler(self)
    }
}

/// Register an event handler that runs when a wry event is processed. Unlike most callback hooks,
/// this runs outside of the virtual dom context.
///
/// The handler may take either form (see [`IntoWryEventHandler`]):
///
/// ```rust, ignore
/// // Stays on the VirtualDom thread; does NOT need to be `Send`. Good for updating signals.
/// use_wry_event_handler(|event: &Event<_>| {
///     if let Event::WindowEvent { event: WindowEvent::Focused(focused), .. } = event {
///         // ...
///     }
/// });
///
/// // Runs on the main event loop thread with the `EventLoopWindowTarget`; must be `Send`.
/// use_wry_event_handler(|event: &Event<_>, target: &_| {
///     // ...
/// });
/// ```
///
/// Capturing a [`DesktopContext`] in the target-taking form is rejected because that handler is
/// moved to the main event loop thread. Use the no-target form if you need to call blocking
/// [`DesktopContext`] APIs.
///
/// ```rust, compile_fail
/// use dioxus_desktop::{tao::event::Event, use_window, use_wry_event_handler};
///
/// fn app() {
///     let desktop = use_window();
///     use_wry_event_handler(move |_event: &Event<()>, _target: &_| {
///         desktop.set_title("will not compile");
///     });
/// }
/// ```
///
/// The handler is removed automatically when the component is dropped.
pub fn use_wry_event_handler<Marker>(
    handler: impl IntoWryEventHandler<Marker> + 'static,
) -> WryEventHandler {
    use_hook_with_cleanup(
        move || handler.into_wry_event_handler(),
        move |handler| handler.remove(),
    )
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
fn use_dom_event_handler(
    mut handler: impl FnMut(UserWindowEvent) + 'static,
    mut forward_event: impl FnMut(&UserWindowEvent) -> Option<UserWindowEvent> + Send + 'static,
) -> WryEventHandler {
    let runtime = Runtime::current();
    let scope_id = current_scope_id();
    use_hook_with_cleanup(
        move || {
            let window = window();
            let Some(registry) = window.callback_registry() else {
                tracing::warn!(
                    "cannot register a dom event handler: this window's VirtualDom is not running"
                );
                return crate::WryEventHandler::new(usize::MAX);
            };
            let dom_handler = registry.register(move |event: UserWindowEvent| {
                runtime.in_scope(scope_id, || handler(event));
            });
            let dom_tx = window.dom_event_sender();
            window
                .create_wry_event_handler_with_user_event(move |event, _| {
                    let Event::UserEvent(event) = event else {
                        return;
                    };
                    let Some(event) = forward_event(event) else {
                        return;
                    };
                    let _ = dom_tx.send(VirtualDomEvent::RunCallback(Box::new(move |registry| {
                        registry.invoke(dom_handler, event);
                    })));
                })
                .with_dom_handler(dom_handler)
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
        move |event| {
            if let UserWindowEventVariant::MudaMenuEvent(event) = event.variant() {
                handler(event);
            }
        },
        |event| match event.variant() {
            UserWindowEventVariant::MudaMenuEvent(event) => {
                Some(UserWindowEvent::muda_menu_event(event.clone()))
            }
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
        move |event| {
            if let UserWindowEventVariant::TrayMenuEvent(event) = event.variant() {
                handler(event);
            }
        },
        |event| match event.variant() {
            UserWindowEventVariant::TrayMenuEvent(event) => {
                Some(UserWindowEvent::tray_menu_event(event.clone()))
            }
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
        move |event| {
            if let UserWindowEventVariant::TrayIconEvent(event) = event.variant() {
                handler(event);
            }
        },
        |event| match event.variant() {
            UserWindowEventVariant::TrayIconEvent(event) => {
                Some(UserWindowEvent::tray_icon_event(event.clone()))
            }
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
            if let Ok((shortcut_handle, dom_id)) = handle {
                window().remove_dom_shortcut(shortcut_handle, dom_id);
            }
        },
    )
    .map(|(handle, _)| handle)
}
