mod use_context;
mod use_effect;
mod use_reducer;
mod use_ref;
mod use_state;

pub use use_context::*;
pub use use_effect::*;
pub use use_reducer::*;
pub use use_ref::*;
pub use use_state::*;

use crate::{HookUpdater, CURRENT_HOOK};
use std::cell::RefCell;
use std::ops::DerefMut;
use std::rc::Rc;

pub fn use_hook<InternalHook: 'static, Output, Tear: FnOnce(&mut InternalHook) -> () + 'static>(
    initializer: impl FnOnce() -> InternalHook,
    runner: impl FnOnce(&mut InternalHook, HookUpdater) -> Output,
    tear_down: Tear,
) -> Output {
    // Extract current hook
    let updater = CURRENT_HOOK.with(|hook_state_holder| {
        let mut hook_state_holder = hook_state_holder
            .try_borrow_mut()
            .expect("Nested hooks not supported");

        let mut hook_state = hook_state_holder
            .as_mut()
            .expect("No current hook. Hooks can only be called inside function components");

        // Determine which hook position we're at and increment for the next hook
        let hook_pos = hook_state.counter;
        hook_state.counter += 1;

        // Initialize hook if this is the first call
        if hook_pos >= hook_state.hooks.len() {
            let initial_state = Rc::new(RefCell::new(initializer()));
            hook_state.hooks.push(initial_state.clone());
            hook_state.destroy_listeners.push(Box::new(move || {
                let mut is = initial_state.borrow_mut();
                let ihook = is.deref_mut();
                tear_down(ihook);
            }));
        }

        let hook = hook_state
            .hooks
            .get(hook_pos)
            .expect("Not the same number of hooks. Hooks must not be called conditionally")
            .clone();

        HookUpdater {
            hook,
            process_message: hook_state.process_message.clone(),
        }
    });

    // Execute the actual hook closure we were given. Let it mutate the hook state and let
    // it create a callback that takes the mutable hook state.
    let mut hook = updater.hook.borrow_mut();
    let hook: &mut InternalHook = hook
        .downcast_mut()
        .expect("Incompatible hook type. Hooks must always be called in the same order");

    runner(hook, updater.clone())
}
