use dioxus_core::{ScopeState, TaskId};
use std::{cell::Cell, future::Future, rc::Rc};

pub fn use_future<'a, T: 'static, F: Future<Output = T> + 'static>(
    cx: &'a ScopeState,
    new_fut: impl FnOnce() -> F,
) -> &'a UseFuture<T> {
    let state = cx.use_hook(move |_| {
        //
        UseFuture {
            update: cx.schedule_update(),
            needs_regen: Cell::new(true),
            slot: Rc::new(Cell::new(None)),
            value: None,
            task: None,
        }
    });

    if let Some(value) = state.slot.take() {
        state.value = Some(value);
        state.task = None;
    }

    if state.needs_regen.get() {
        // We don't need regen anymore
        state.needs_regen.set(false);

        // Create the new future
        let fut = new_fut();

        // Clone in our cells
        let slot = state.slot.clone();
        let updater = state.update.clone();

        state.task = Some(cx.push_future(async move {
            let res = fut.await;
            slot.set(Some(res));
            updater();
        }));
    }

    state
}

pub struct UseFuture<T> {
    update: Rc<dyn Fn()>,
    needs_regen: Cell<bool>,
    value: Option<T>,
    slot: Rc<Cell<Option<T>>>,
    task: Option<TaskId>,
}

impl<T> UseFuture<T> {
    pub fn restart(&self) {
        self.needs_regen.set(true);
        (self.update)();
    }

    // clears the value in the future slot without starting the future over
    pub fn clear(&self) -> Option<T> {
        (self.update)();
        self.slot.replace(None)
    }

    // Manually set the value in the future slot without starting the future over
    pub fn set(&self, new_value: T) {
        self.slot.set(Some(new_value));
        self.needs_regen.set(true);
        (self.update)();
    }

    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }
}
