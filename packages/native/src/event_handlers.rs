use slab::Slab;
use std::cell::RefCell;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::WindowId};

/// The unique identifier of a window event handler. This can be used to later remove the handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WinitEventHandler(usize);

impl WinitEventHandler {
    /// Unregister this event handler from the window
    pub fn remove(&self) {
        WINDOW_EVENT_HANDLERS.with(|h| {
            h.borrow_mut().remove(*self);
        });
    }
}

struct WinitWindowEventHandlerInner {
    window_id: WindowId,

    #[allow(clippy::type_complexity)]
    handler: Box<dyn FnMut(&WindowEvent, &dyn ActiveEventLoop) + 'static>,
}

pub(crate) struct WindowEventHandlers {
    handlers: Slab<WinitWindowEventHandlerInner>,
}

impl Default for WindowEventHandlers {
    fn default() -> Self {
        Self {
            handlers: Slab::new(),
        }
    }
}

impl WindowEventHandlers {
    pub(crate) fn add(
        &mut self,
        window_id: WindowId,
        handler: impl FnMut(&WindowEvent, &dyn ActiveEventLoop) + 'static,
    ) -> WinitEventHandler {
        WinitEventHandler(self.handlers.insert(WinitWindowEventHandlerInner {
            window_id,
            handler: Box::new(handler),
        }))
    }

    pub(crate) fn remove(&mut self, id: WinitEventHandler) {
        self.handlers.try_remove(id.0);
    }

    pub fn apply_event(
        &mut self,
        window_id: WindowId,
        event: &WindowEvent,
        target: &dyn ActiveEventLoop,
    ) {
        for (_, handler) in self.handlers.iter_mut() {
            if handler.window_id != window_id {
                continue;
            }
            (handler.handler)(event, target)
        }
    }
}

thread_local! {
    pub(crate) static WINDOW_EVENT_HANDLERS: RefCell<WindowEventHandlers> =
        RefCell::new(WindowEventHandlers::default());
}
