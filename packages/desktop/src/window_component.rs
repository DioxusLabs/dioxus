use crate::{
    Config, DesktopContext, app::SharedContext, desktop_context::PendingWindowCancellation,
    document::DesktopDocument, event_handlers::WindowCloseHandler, window,
};
use dioxus_core::view::ViewExt;
use dioxus_core::{
    Element, EventHandler, Portal, PortalProps, Properties, RenderTargetId, Runtime, VNode,
    provide_context, schedule_update, spawn, use_hook, use_hook_with_cleanup,
};
use dioxus_document::Document;
use dioxus_history::{History, MemoryHistory};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// Properties for the [`Window()`] component.
///
/// Use the `config` prop to set the initial native window configuration, the
/// `onclose` prop to handle a user-initiated close, and children to choose what
/// the window renders. Configuration is applied when the window is opened.
#[derive(dioxus_core_macro::Props, Clone)]
pub struct WindowProps {
    #[props(into, default)]
    config: InitialWindowConfig,
    onclose: Option<EventHandler<()>>,
    children: Element,
}

impl PartialEq for WindowProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

#[derive(Clone, Default)]
#[doc(hidden)]
pub struct InitialWindowConfig(Rc<RefCell<Option<Config>>>);

impl From<Config> for InitialWindowConfig {
    fn from(config: Config) -> Self {
        Self(Rc::new(RefCell::new(Some(config))))
    }
}

#[derive(Clone)]
struct WindowProviders {
    context: DesktopContext,
    document: Rc<dyn Document>,
    history: Rc<dyn History>,
}

#[derive(Clone)]
struct WindowState {
    target_id: RenderTargetId,
    shared: Rc<SharedContext>,
    pending_cancellation: PendingWindowCancellation,
    providers: Rc<RefCell<Option<WindowProviders>>>,
    closed: Rc<Cell<bool>>,
    onclose: Rc<RefCell<Option<EventHandler<()>>>>,
    close_handler: Rc<RefCell<Option<WindowCloseHandler>>>,
}

impl WindowState {
    fn cancel_pending_webview(&self) -> bool {
        self.pending_cancellation.cancel();

        let mut pending_webviews = self.shared.pending_webviews.borrow_mut();
        let Some(index) = pending_webviews.iter().position(|pending| {
            pending.matches_pending_window(self.target_id, &self.pending_cancellation)
        }) else {
            return false;
        };

        pending_webviews.remove(index);
        true
    }

    fn remove_close_handler(&self) {
        let Some(handler) = self.close_handler.borrow_mut().take() else {
            return;
        };
        if let Some(providers) = self.providers.borrow().as_ref() {
            providers
                .context
                .shared
                .window_close_handlers
                .remove(handler);
        }
    }

    /// Close the underlying window after the `Window` component has been
    /// removed from the tree. At this point the portal feeding `target_id` has
    /// already been torn down, so the render target can be reclaimed before the
    /// app drops the native webview and its `WryQueue`.
    ///
    /// Runs from the `Window` scope's drop cleanup, after its rendered subtree
    /// (the portal feeding `target_id`) has already been torn down, so the
    /// render target is empty and safe to reclaim. `try_current` is `None`
    /// during full `VirtualDom` teardown, where the whole arena is dropped
    /// anyway, so the reclaim is simply skipped.
    fn close_window(&self) {
        self.remove_close_handler();
        let pending_removed = self.cancel_pending_webview();
        let providers = self.providers.borrow_mut().take();
        let can_reclaim_target = pending_removed || providers.is_some() || self.closed.get();

        if let Some(providers) = providers {
            providers.context.close();
        }
        if can_reclaim_target {
            let Some(runtime) = Runtime::try_current() else {
                return;
            };
            runtime.remove_render_target(self.target_id);
        }
    }

    fn release_closed_window(&self) {
        self.remove_close_handler();
        if let Some(providers) = self.providers.borrow_mut().take() {
            // Queue native teardown after this render removes the portal. The
            // app reclaims the render target when it receives this close event.
            providers.context.close();
        }
    }
}

/// Render children into a separate desktop window.
///
/// `Window` behaves like an ordinary component from the app's point of view:
/// it accepts children, can read the same context as its parent, and can update
/// in response to the same state changes. The difference is that its children
/// are displayed in their own native desktop window.
///
/// The optional `config` prop customizes the window when it is opened. The
/// optional `onclose` handler runs when the user closes the native window.
///
/// ```rust,ignore
/// use dioxus::prelude::*;
/// use dioxus::desktop::{Config, Window, WindowBuilder};
///
/// fn App() -> Element {
///     rsx! {
///         Window {
///             config: Config::new().with_window(WindowBuilder::new().with_title("Inspector")),
///             onclose: move |_| tracing::info!("inspector closed"),
///             div { "Tools" }
///         }
///     }
/// }
/// ```
#[allow(non_snake_case)]
pub fn Window(props: WindowProps) -> Element {
    let schedule_update = schedule_update();
    let state = {
        let config = props.config.0.clone();
        use_hook(move || {
            let providers = Rc::new(RefCell::new(None));
            let closed = Rc::new(Cell::new(false));
            let onclose = Rc::new(RefCell::new(None::<EventHandler<()>>));
            let close_handler = Rc::new(RefCell::new(None));
            let desktop_context = window();
            let shared = desktop_context.shared.clone();
            let pending =
                desktop_context.new_window(config.borrow_mut().take().unwrap_or_default());
            let target_id = pending.target_id();
            let pending_cancellation = pending.cancellation();
            let providers_for_task = providers.clone();
            let closed_for_task = closed.clone();
            let onclose_for_task = onclose.clone();
            let close_handler_for_task = close_handler.clone();
            let pending_cancellation_for_task = pending_cancellation.clone();

            spawn(async move {
                let Ok(resolved_context) = pending.try_resolve().await else {
                    return;
                };
                if pending_cancellation_for_task.is_canceled() {
                    resolved_context.close();
                    if let Some(runtime) = Runtime::try_current() {
                        runtime.remove_render_target(target_id);
                    }
                    return;
                }
                let window_id = resolved_context.window.id();
                let closed_for_close_handler = closed_for_task.clone();
                let schedule_update_for_close_handler = schedule_update.clone();
                let close_handler =
                    resolved_context
                        .shared
                        .window_close_handlers
                        .add(window_id, move || {
                            let was_closed = closed_for_close_handler.replace(true);
                            if !was_closed {
                                if let Some(onclose) = *onclose_for_task.borrow() {
                                    onclose.call(());
                                }
                                schedule_update_for_close_handler();
                            }
                        });

                close_handler_for_task.borrow_mut().replace(close_handler);
                providers_for_task.borrow_mut().replace(WindowProviders {
                    document: Rc::new(DesktopDocument::new(resolved_context.clone())),
                    history: Rc::new(MemoryHistory::default()),
                    context: resolved_context,
                });
                schedule_update();
            });

            WindowState {
                target_id,
                shared,
                pending_cancellation,
                providers,
                closed,
                onclose,
                close_handler,
            }
        })
    };
    state.onclose.replace(props.onclose);

    use_hook_with_cleanup(
        {
            let state = state.clone();
            move || state
        },
        |state| {
            state.close_window();
        },
    );

    if state.closed.get() {
        state.release_closed_window();
        return VNode::empty();
    }

    let Some(providers) = state.providers.borrow().clone() else {
        return VNode::empty();
    };

    portal_element(
        state.target_id,
        context_provider_element(providers, props.children.clone()),
    )
}

#[derive(dioxus_core_macro::Props, Clone)]
struct WindowContextProviderProps {
    providers: WindowProviders,
    children: Element,
}

impl PartialEq for WindowContextProviderProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

#[allow(non_snake_case)]
fn WindowContextProvider(props: WindowContextProviderProps) -> Element {
    provide_context(props.providers.context);
    provide_context(props.providers.document);
    provide_context(props.providers.history);
    props.children
}

fn context_provider_element(providers: WindowProviders, children: Element) -> Element {
    Element::Ok(
        <WindowContextProviderProps as Properties>::component_builder(WindowContextProvider)
            .providers(providers)
            .children(children)
            .build()
            .into_vcomponent()
            .into_vnode(),
    )
}

fn portal_element(target: RenderTargetId, children: Element) -> Element {
    Element::Ok(
        <PortalProps as Properties>::component_builder(Portal)
            .target(target)
            .children(children)
            .build()
            .into_vcomponent()
            .into_vnode(),
    )
}
