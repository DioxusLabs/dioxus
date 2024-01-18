use crate::{ipc::UserWindowEvent, window};
use slab::Slab;
use std::cell::RefCell;
use tao::{event::Event, event_loop::EventLoopWindowTarget, window::WindowId};

/// The unique identifier of a window event handler. This can be used to later remove the handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WryEventHandler(pub(crate) usize);

impl WryEventHandler {
    /// Unregister this event handler from the window
    pub fn remove(&self) {
        window().shared.event_handlers.remove(*self)
    }
}

#[derive(Default)]
pub struct WindowEventHandlers {
    handlers: RefCell<Slab<WryWindowEventHandlerInner>>,
}

struct WryWindowEventHandlerInner {
    window_id: WindowId,

    #[allow(clippy::type_complexity)]
    handler:
        Box<dyn FnMut(&Event<UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>) + 'static>,
}

impl WindowEventHandlers {
    pub(crate) fn add(
        &self,
        window_id: WindowId,
        handler: impl FnMut(&Event<UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>) + 'static,
    ) -> WryEventHandler {
        WryEventHandler(
            self.handlers
                .borrow_mut()
                .insert(WryWindowEventHandlerInner {
                    window_id,
                    handler: Box::new(handler),
                }),
        )
    }

    pub(crate) fn remove(&self, id: WryEventHandler) {
        self.handlers.borrow_mut().try_remove(id.0);
    }

    pub fn apply_event(
        &self,
        event: &Event<UserWindowEvent>,
        target: &EventLoopWindowTarget<UserWindowEvent>,
    ) {
        for (_, handler) in self.handlers.borrow_mut().iter_mut() {
            // if this event does not apply to the window this listener cares about, return
            if let Event::WindowEvent { window_id, .. } = event {
                if *window_id != handler.window_id {
                    return;
                }
            }
            (handler.handler)(event, target)
        }
    }
}
