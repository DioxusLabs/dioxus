use crate::{ipc::UserWindowEvent, window};
use slab::Slab;
use std::{cell::RefCell, rc::Rc};
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

/// The unique identifier of a window close handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct WindowCloseHandler(pub(crate) usize);

#[derive(Default)]
pub(crate) struct WindowCloseHandlers {
    handlers: RefCell<Slab<WindowCloseHandlerInner>>,
}

struct WindowCloseHandlerInner {
    window_id: WindowId,
    handler: Rc<dyn Fn() + 'static>,
}

impl WindowCloseHandlers {
    pub(crate) fn add(
        &self,
        window_id: WindowId,
        handler: impl Fn() + 'static,
    ) -> WindowCloseHandler {
        WindowCloseHandler(self.handlers.borrow_mut().insert(WindowCloseHandlerInner {
            window_id,
            handler: Rc::new(handler),
        }))
    }

    pub(crate) fn remove(&self, id: WindowCloseHandler) {
        self.handlers.borrow_mut().try_remove(id.0);
    }

    pub(crate) fn notify(&self, window_id: WindowId) -> bool {
        let handlers: Vec<_> = self
            .handlers
            .borrow()
            .iter()
            .filter(|(_, handler)| handler.window_id == window_id)
            .map(|(_, handler)| handler.handler.clone())
            .collect();

        let handled = !handlers.is_empty();

        for handler in handlers {
            handler();
        }

        handled
    }
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
            // Avoid interacting with the runtime while something else is using it
            #[cfg(target_os = "android")]
            let _lock = crate::android_sync_lock::android_runtime_lock();

            // if this event does not apply to the window this listener cares about, continue
            if let Event::WindowEvent { window_id, .. } = event {
                if *window_id != handler.window_id {
                    continue;
                }
            }

            (handler.handler)(event, target)
        }
    }
}
