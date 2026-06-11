#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
use crate::dom_thread::DomCallbackId;
use crate::{ipc::UserWindowEvent, window};
use slab::Slab;
use std::cell::RefCell;
use tao::{event::Event, event_loop::EventLoopWindowTarget, window::WindowId};

/// The unique identifier of a window event handler. This can be used to later remove the handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WryEventHandler {
    pub(crate) id: usize,
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub(crate) dom_handler: Option<DomCallbackId>,
}

impl WryEventHandler {
    pub(crate) fn new(id: usize) -> Self {
        Self {
            id,
            #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
            dom_handler: None,
        }
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub(crate) fn with_dom_handler(mut self, dom_handler: DomCallbackId) -> Self {
        self.dom_handler = Some(dom_handler);
        self
    }

    /// Unregister this event handler from the window
    pub fn remove(&self) {
        window().remove_wry_event_handler(*self);
    }
}

#[derive(Default)]
pub struct WindowEventHandlers {
    handlers: RefCell<Slab<WryWindowEventHandlerInner>>,
}

struct WryWindowEventHandlerInner {
    window_id: WindowId,

    #[allow(clippy::type_complexity)]
    handler: Box<
        dyn for<'a> FnMut(
                Event<'a, UserWindowEvent>,
                &EventLoopWindowTarget<UserWindowEvent>,
            ) -> Event<'a, UserWindowEvent>
            + 'static,
    >,
}

impl WindowEventHandlers {
    pub(crate) fn add_raw(
        &self,
        window_id: WindowId,
        handler: impl for<'a> FnMut(
            Event<'a, UserWindowEvent>,
            &EventLoopWindowTarget<UserWindowEvent>,
        ) -> Event<'a, UserWindowEvent>
        + 'static,
    ) -> WryEventHandler {
        WryEventHandler::new(
            self.handlers
                .borrow_mut()
                .insert(WryWindowEventHandlerInner {
                    window_id,
                    handler: Box::new(handler),
                }),
        )
    }

    pub(crate) fn add(
        &self,
        window_id: WindowId,
        handler: impl for<'a> FnMut(&Event<'a, ()>, &EventLoopWindowTarget<UserWindowEvent>) + 'static,
    ) -> WryEventHandler {
        let handler = wrap_generic_event_handler(handler);
        self.add_raw(window_id, handler)
    }

    pub(crate) fn add_with_user_event(
        &self,
        window_id: WindowId,
        handler: impl for<'a> FnMut(
            &Event<'a, UserWindowEvent>,
            &EventLoopWindowTarget<UserWindowEvent>,
        ) + 'static,
    ) -> WryEventHandler {
        let handler = wrap_event_handler(handler);
        self.add_raw(window_id, handler)
    }

    pub(crate) fn remove(&self, id: WryEventHandler) {
        self.handlers.borrow_mut().try_remove(id.id);
    }

    pub fn apply_event<'a>(
        &self,
        mut event: Event<'a, UserWindowEvent>,
        target: &EventLoopWindowTarget<UserWindowEvent>,
    ) -> Event<'a, UserWindowEvent> {
        for (_, handler) in self.handlers.borrow_mut().iter_mut() {
            // Avoid interacting with the runtime while something else is using it
            #[cfg(target_os = "android")]
            let _lock = crate::android_sync_lock::android_runtime_lock();

            // if this event does not apply to the window this listener cares about, continue
            if let Event::WindowEvent { window_id, .. } = &event {
                if *window_id != handler.window_id {
                    continue;
                }
            }

            event = (handler.handler)(event, target);
        }
        event
    }
}

/// Run a closure if this is not a user event
fn with_user_event<'a, 'b>(
    event: Event<'a, UserWindowEvent>,
    with_generic_event: impl FnOnce(&Event<'a, ()>),
) -> Event<'a, UserWindowEvent> {
    let non_user_event: Result<Event<'a, ()>, Event<'a, UserWindowEvent>> =
        event.map_nonuser_event();
    match non_user_event {
        Ok(event) => {
            with_generic_event(&event);
            event
                .map_nonuser_event()
                .expect("non-user event stays non-user after being passed to the handler")
        }
        Err(event) => event,
    }
}

/// Turn a closure that takes a generic event into one that takes a user event, by ignoring the user event data.
fn wrap_generic_event_handler(
    mut handler: impl for<'a> FnMut(&Event<'a, ()>, &EventLoopWindowTarget<UserWindowEvent>) + 'static,
) -> impl for<'a> FnMut(
    Event<'a, UserWindowEvent>,
    &EventLoopWindowTarget<UserWindowEvent>,
) -> Event<'a, UserWindowEvent>
+ 'static {
    move |event: Event<UserWindowEvent>, target| -> Event<UserWindowEvent> {
        with_user_event(event, |generic_event| {
            handler(generic_event, target);
        })
    }
}

/// Turn a closure that takes a generic event into one that takes a user event, by ignoring the user event data.
fn wrap_event_handler(
    mut handler: impl for<'a> FnMut(
        &Event<'a, UserWindowEvent>,
        &EventLoopWindowTarget<UserWindowEvent>,
    ) + 'static,
) -> impl for<'a> FnMut(
    Event<'a, UserWindowEvent>,
    &EventLoopWindowTarget<UserWindowEvent>,
) -> Event<'a, UserWindowEvent>
+ 'static {
    move |event: Event<UserWindowEvent>, target| -> Event<UserWindowEvent> {
        handler(&event, target);
        event
    }
}
