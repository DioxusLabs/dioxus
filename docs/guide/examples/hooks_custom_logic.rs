#![allow(unused)]

use dioxus::prelude::*;

fn main() {}

// ANCHOR: use_state
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
struct UseState<T> {
    value: Rc<RefCell<T>>,
    update: Arc<dyn Fn()>,
}

fn my_use_state<T: 'static>(cx: &ScopeState, init: impl FnOnce() -> T) -> &UseState<T> {
    cx.use_hook(|| {
        // The update function will trigger a re-render in the component cx is attached to
        let update = cx.schedule_update();
        // Create the initial state
        let value = Rc::new(RefCell::new(init()));

        UseState { value, update }
    })
}

impl<T: Clone> UseState<T> {
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
// ANCHOR_END: use_state

// ANCHOR: use_context
pub fn use_context<T: 'static + Clone>(cx: &ScopeState) -> Option<&T> {
    cx.use_hook(|| cx.consume_context::<T>()).as_ref()
}

pub fn use_context_provider<T: 'static + Clone>(cx: &ScopeState, f: impl FnOnce() -> T) -> &T {
    cx.use_hook(|| {
        let val = f();
        // Provide the context state to the scope
        cx.provide_context(val.clone());
        val
    })
}

// ANCHOR_END: use_context
