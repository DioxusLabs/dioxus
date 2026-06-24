use crate::{
    DesktopContext,
    desktop_context::PendingWindowCancellation,
    desktop_state::{ComponentWindowRegistration, DesktopAppContext},
};
use dioxus_core::{EventHandler, RenderTargetId};
use dioxus_document::Document;
use dioxus_history::History;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct WindowProviders {
    pub(crate) document: Rc<dyn Document>,
    pub(crate) history: Rc<dyn History>,
    pub(crate) context: DesktopContext,
}

pub(crate) enum ComponentWindowRenderState {
    Waiting,
    Render {
        target_id: RenderTargetId,
        providers: WindowProviders,
    },
}

/// A pending native close that the [`Window`](crate::Window) component should honor.
pub(crate) struct CloseRequest {
    pub(crate) onclose: Option<EventHandler<()>>,
}

pub(crate) struct ComponentWindowLifecycle {
    target_id: RenderTargetId,
    app_context: Rc<DesktopAppContext>,
    pending_cancellation: PendingWindowCancellation,
    providers: Option<WindowProviders>,
    close_registration: Option<ComponentWindowRegistration>,
    onclose: Option<EventHandler<()>>,
    close_requested: bool,
    close_dispatched: bool,
    native_destroyed: bool,
}

impl ComponentWindowLifecycle {
    pub(crate) fn pending(
        target_id: RenderTargetId,
        app_context: Rc<DesktopAppContext>,
        pending_cancellation: PendingWindowCancellation,
    ) -> Self {
        Self {
            target_id,
            app_context,
            pending_cancellation,
            providers: None,
            close_registration: None,
            onclose: None,
            close_requested: false,
            close_dispatched: false,
            native_destroyed: false,
        }
    }

    pub(crate) fn set_onclose(&mut self, onclose: Option<EventHandler<()>>) {
        self.onclose = onclose;
    }

    pub(crate) fn resolve_pending(
        &mut self,
        providers: WindowProviders,
        close_registration: ComponentWindowRegistration,
    ) {
        self.providers = Some(providers);
        self.close_registration = Some(close_registration);
    }

    /// Mark the window as closing in response to a native close request.
    ///
    /// Returns the [`CloseRequest`] to honor the first time close is requested, or `None` if the
    /// window is already closing or has been destroyed.
    pub(crate) fn request_close(&mut self) -> Option<CloseRequest> {
        if self.close_requested || self.native_destroyed {
            return None;
        }
        self.close_requested = true;
        Some(CloseRequest {
            onclose: self.onclose,
        })
    }

    pub(crate) fn native_destroyed(&mut self) -> bool {
        if self.native_destroyed {
            return false;
        }
        self.native_destroyed = true;
        true
    }

    pub(crate) fn release_from_component_drop(&mut self) {
        self.pending_cancellation.cancel();
        self.close_registration.take();

        if let Some(providers) = self.providers.take()
            && !self.native_destroyed
        {
            providers.context.close();
        } else if self.providers.is_none() {
            self.app_context
                .proxy
                .send_event(crate::ipc::UserWindowEvent::Poll)
                .ok();
        }
    }

    pub(crate) fn prepare_to_render(&mut self) -> ComponentWindowRenderState {
        if self.close_requested || self.native_destroyed {
            if self.close_requested
                && !self.close_dispatched
                && let Some(providers) = &self.providers
            {
                self.close_dispatched = true;
                _ = providers.context.app.proxy.send_event(
                    crate::ipc::UserWindowEvent::DestroyWindow(providers.context.id()),
                );
            }
            return ComponentWindowRenderState::Waiting;
        }

        match self.providers.clone() {
            Some(providers) => ComponentWindowRenderState::Render {
                target_id: self.target_id,
                providers,
            },
            None => ComponentWindowRenderState::Waiting,
        }
    }
}
