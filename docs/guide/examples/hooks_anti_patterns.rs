#![allow(unused)]

use dioxus::prelude::*;

fn main() {}

// ANCHOR: non_clone_state
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

struct UseState<'a, T> {
    value: &'a RefCell<T>,
    update: Arc<dyn Fn()>,
}

fn my_use_state<T: 'static>(cx: &ScopeState, init: impl FnOnce() -> T) -> UseState<T> {
    // The update function will trigger a re-render in the component cx is attached to
    let update = cx.schedule_update();
    // Create the initial state
    let value = cx.use_hook(|| RefCell::new(init()));

    UseState { value, update }
}

impl<T: Clone> UseState<'_, T> {
    fn get(&self) -> T {
        self.value.borrow().clone()
    }

    fn set(&self, value: T) {
        // Update the state
        *self.value.borrow_mut() = value;
        // Trigger a re-render on the component the state is from
        (self.update)();
    }
}
// ANCHOR_END: non_clone_state
