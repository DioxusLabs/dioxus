use crate::{
    Config,
    document::DesktopDocument,
    window,
    window_lifecycle::{ComponentWindowLifecycle, ComponentWindowRenderState, WindowProviders},
};
use dioxus_core::view::ViewExt;
use dioxus_core::{
    Element, EventHandler, Portal, PortalProps, Properties, RenderTargetId, VNode, provide_context,
    schedule_update, spawn, use_hook, use_hook_with_cleanup,
};
use dioxus_history::MemoryHistory;
use std::{cell::RefCell, rc::Rc};

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
struct WindowState(Rc<RefCell<ComponentWindowState>>);

struct ComponentWindowState {
    lifecycle: ComponentWindowLifecycle,
    onclose: Option<EventHandler<()>>,
}

struct CloseRequest {
    onclose: Option<EventHandler<()>>,
}

impl WindowState {
    fn set_onclose(&self, onclose: Option<EventHandler<()>>) {
        self.0.borrow_mut().onclose = onclose;
    }

    fn resolve_pending(
        &self,
        providers: WindowProviders,
        close_registration: crate::desktop_state::ComponentWindowRegistration,
    ) {
        self.0
            .borrow_mut()
            .lifecycle
            .resolve_pending(providers, close_registration);
    }

    fn release_canceled_resolved_window(&self, context: crate::DesktopContext) {
        self.0
            .borrow_mut()
            .lifecycle
            .release_canceled_resolved_window(context);
    }

    fn request_close(&self) -> Option<CloseRequest> {
        let mut state = self.0.borrow_mut();
        state.lifecycle.request_close().then_some(CloseRequest {
            onclose: state.onclose,
        })
    }

    fn native_destroyed(&self) -> bool {
        self.0.borrow_mut().lifecycle.native_destroyed()
    }

    fn release_from_component_drop(&self) {
        self.0.borrow_mut().lifecycle.release_from_component_drop();
    }

    fn prepare_to_render(&self) -> ComponentWindowRenderState {
        self.0.borrow_mut().lifecycle.prepare_to_render()
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
            let desktop_context = window();
            let app_context = desktop_context.app_context().clone();
            let pending =
                desktop_context.new_window(config.borrow_mut().take().unwrap_or_default());
            let target_id = pending.target_id();
            let pending_cancellation = pending.cancellation();
            let state = WindowState(Rc::new(RefCell::new(ComponentWindowState {
                lifecycle: ComponentWindowLifecycle::pending(
                    target_id,
                    app_context,
                    pending_cancellation.clone(),
                ),
                onclose: None,
            })));
            let state_for_task = state.clone();
            let pending_cancellation_for_task = pending_cancellation.clone();

            spawn(async move {
                let Ok(resolved_context) = pending.try_resolve().await else {
                    return;
                };
                if pending_cancellation_for_task.is_canceled() {
                    state_for_task.release_canceled_resolved_window(resolved_context);
                    return;
                }
                let window_id = resolved_context.window.id();
                let schedule_update_for_close_handler = schedule_update.clone();
                let schedule_update_for_destroyed = schedule_update.clone();
                let state_for_close_handler = state_for_task.clone();
                let state_for_destroyed_handler = state_for_task.clone();
                let app_context = resolved_context.app_context().clone();
                let close_registration = app_context.register_component_window(
                    window_id,
                    move || {
                        let Some(close_request) = state_for_close_handler.request_close() else {
                            return;
                        };
                        if let Some(onclose) = close_request.onclose {
                            onclose.call(());
                        }
                        schedule_update_for_close_handler();
                    },
                    move || {
                        if state_for_destroyed_handler.native_destroyed() {
                            schedule_update_for_destroyed();
                        }
                    },
                );

                state_for_task.resolve_pending(
                    WindowProviders {
                        document: Rc::new(DesktopDocument::new(resolved_context.clone())),
                        history: Rc::new(MemoryHistory::default()),
                        context: resolved_context,
                    },
                    close_registration,
                );
                schedule_update();
            });

            state
        })
    };
    state.set_onclose(props.onclose);

    use_hook_with_cleanup(
        {
            let state = state.clone();
            move || state
        },
        |state| {
            state.release_from_component_drop();
        },
    );

    match state.prepare_to_render() {
        ComponentWindowRenderState::Waiting => VNode::empty(),
        ComponentWindowRenderState::Render {
            target_id,
            providers,
        } => portal_element(
            target_id,
            context_provider_element(providers, props.children.clone()),
        ),
    }
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
