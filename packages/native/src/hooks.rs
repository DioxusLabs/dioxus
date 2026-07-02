use crate::event_handlers::{WINDOW_EVENT_HANDLERS, WinitEventHandler};

use dioxus_core::{Runtime, current_scope_id, use_hook_with_cleanup};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop};

/// Register an event handler that runs when a winit event is processed.
pub fn use_winit_event_handler(
    mut handler: impl FnMut(&WindowEvent, &dyn ActiveEventLoop) + 'static,
) -> WinitEventHandler {
    let runtime = Runtime::current();
    let scope_id = current_scope_id();
    let window_id = crate::use_window().id();

    use_hook_with_cleanup(
        move || {
            WINDOW_EVENT_HANDLERS.with(|h| {
                h.borrow_mut().add(window_id, move |event, target| {
                    runtime.in_scope(scope_id, || handler(event, target))
                })
            })
        },
        move |handler| handler.remove(),
    )
}
