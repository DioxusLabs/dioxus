use crate::event_handlers::{WindowEventHandlers, WinitEventHandlerId};

use dioxus_core::{Runtime, consume_context, current_scope_id, use_hook_with_cleanup};
use std::rc::Rc;
use winit::{
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
};

/// Register an event handler that runs when a winit event is processed.
pub fn use_window_event(
    mut handler: impl FnMut(&WindowEvent, &dyn ActiveEventLoop) + 'static,
) -> WinitEventHandlerId {
    let runtime = Runtime::current();
    let scope_id = current_scope_id();
    let window_id = crate::use_window().id();

    use_hook_with_cleanup(
        move || {
            let handlers: Rc<WindowEventHandlers> = consume_context();
            handlers.add(window_id, move |event, target| {
                runtime.in_scope(scope_id, || handler(event, target))
            })
        },
        move |handler| handler.remove(),
    )
}

/// Register a handler that runs when the back button is pressed.
///
/// This builds on top of [`use_window_event`]: the back button is delivered by `winit` as a
/// [`WindowEvent::KeyboardInput`] whose logical key is [`NamedKey::BrowserBack`]. This most
/// commonly comes from the Android hardware/system back button, but may also be produced by a
/// keyboard or mouse back key on other platforms. The provided `handler` is called once each
/// time the button is pressed (key repeats are ignored).
///
/// Returns a [`WinitEventHandlerId`] which can be used to remove the handler.
pub fn use_back_button(mut handler: impl FnMut() + 'static) -> WinitEventHandlerId {
    use_window_event(move |event, _target| {
        if let WindowEvent::KeyboardInput { event, .. } = event
            && event.state == ElementState::Pressed
            && !event.repeat
            && event.logical_key == Key::Named(NamedKey::BrowserBack)
        {
            handler();
        }
    })
}
