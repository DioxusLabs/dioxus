use crate::{ipc::UserWindowEvent, window};
use slotmap::SlotMap;
use std::cell::RefCell;
use tao::{event::Event, event_loop::EventLoopWindowTarget, window::WindowId};

slotmap::new_key_type! {
    /// The unique identifier of a window event handler. This can be used to later remove the
    /// handler. Ids are generational: a removed handler's id can never remove a different
    /// handler reusing its slot.
    pub struct WryEventHandler;
}

impl WryEventHandler {
    /// Unregister this event handler from the window
    pub fn remove(&self) {
        window().remove_wry_event_handler(*self);
    }
}

#[derive(Default)]
pub struct WindowEventHandlers {
    handlers: RefCell<SlotMap<WryEventHandler, WryWindowEventHandlerInner>>,
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
        self.handlers
            .borrow_mut()
            .insert(WryWindowEventHandlerInner {
                window_id,
                handler: Box::new(handler),
            })
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
        self.handlers.borrow_mut().remove(id);
    }

    /// Drop every handler `window_id` registered. Runs when the window closes, mirroring the
    /// DOM-side callback purge.
    pub(crate) fn remove_window(&self, window_id: WindowId) {
        self.handlers
            .borrow_mut()
            .retain(|_, handler| handler.window_id != window_id);
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
