use crate::{
    WindowConfig, app,
    document::DesktopDocument,
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
pub struct InitialWindowConfig(Rc<RefCell<Option<WindowConfig>>>);

impl InitialWindowConfig {
    pub(crate) fn from_cell(config: Rc<RefCell<Option<WindowConfig>>>) -> Self {
        Self(config)
    }
}

impl From<WindowConfig> for InitialWindowConfig {
    fn from(config: WindowConfig) -> Self {
        Self(Rc::new(RefCell::new(Some(config))))
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
/// use dioxus::desktop::{Window, WindowBuilder, WindowConfig};
///
/// fn App() -> Element {
///     rsx! {
///         Window {
///             config: WindowConfig::new().with_window(WindowBuilder::new().with_title("Inspector")),
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
            let app_context = app();
            let pending = app_context.new_window(config.borrow_mut().take().unwrap_or_default());
            let target_id = pending.target_id();
            let pending_cancellation = pending.cancellation();
            let state = Rc::new(RefCell::new(ComponentWindowLifecycle::pending(
                target_id,
                app_context,
                pending_cancellation.clone(),
            )));
            let state_for_task = state.clone();
            let pending_cancellation_for_task = pending_cancellation.clone();

            spawn(async move {
                let Ok(resolved_context) = pending.try_resolve().await else {
                    return;
                };
                if pending_cancellation_for_task.is_canceled() {
                    resolved_context.close();
                    return;
                }
                let schedule_update_for_close_handler = schedule_update.clone();
                let schedule_update_for_destroyed = schedule_update.clone();
                let state_for_close_handler = state_for_task.clone();
                let state_for_destroyed_handler = state_for_task.clone();
                let close_registration = resolved_context.register_component_window(
                    move || {
                        let Some(close) = state_for_close_handler.borrow_mut().request_close()
                        else {
                            return;
                        };
                        if let Some(onclose) = close.onclose {
                            onclose.call(());
                        }
                        schedule_update_for_close_handler();
                    },
                    move || {
                        if state_for_destroyed_handler.borrow_mut().native_destroyed() {
                            schedule_update_for_destroyed();
                        }
                    },
                );

                state_for_task.borrow_mut().resolve_pending(
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
    state.borrow_mut().set_onclose(props.onclose);

    use_hook_with_cleanup(
        {
            let state = state.clone();
            move || state
        },
        |state| {
            state.borrow_mut().release_from_component_drop();
        },
    );

    let render_state = state.borrow_mut().prepare_to_render();
    match render_state {
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
