#![allow(missing_docs)]
use dioxus_core::{
    prelude::{spawn, use_before_render, use_drop, use_hook},
    ScopeState, Task,
};
use dioxus_signals::*;
use dioxus_signals::{Readable, Writable};
use futures_util::{future, pin_mut, FutureExt};
use std::{any::Any, cell::Cell, future::Future, pin::Pin, rc::Rc, sync::Arc, task::Poll};

use crate::use_callback;

/// A hook that allows you to spawn a future
///
/// Does not regenerate the future when dependencies change.
pub fn use_future<F>(mut future: impl FnMut() -> F) -> UseFuture
where
    F: Future + 'static,
{
    let state = use_signal(|| UseFutureState::Pending);

    // Create the task inside a copyvalue so we can reset it in-place later
    let task = use_hook(|| {
        let fut = future();
        CopyValue::new(spawn(async move {
            fut.await;
        }))
    });

    /*
    Early returns in dioxus have consequences for use_memo, use_resource, and use_future, etc
    We *don't* want futures to be running if the component early returns. It's a rather weird behavior to have
    use_memo running in the background even if the component isn't hitting those hooks anymore.

    React solves this by simply not having early returns interleave with hooks.
    However, since dioxus allows early returns (since we use them for suspense), we need to solve this problem.


     */
    // Track if this *current* render is the same
    let gen = use_hook(|| CopyValue::new((0, 0)));

    // Early returns will pause this task, effectively
    use_before_render(move || {
        gen.write().0 += 1;
        task.peek().set_active(false);
    });

    // However when we actually run this component, we want to resume the task
    task.peek().set_active(true);
    gen.write().1 += 1;

    // if the gens are different, we need to wake the task
    if gen().0 != gen().1 {
        task.peek().wake();
    }

    use_drop(move || task.peek().stop());

    UseFuture { task, state }
}

pub struct UseFuture {
    task: CopyValue<Task>,
    state: Signal<UseFutureState>,
}

impl UseFuture {
    /// Restart the future with new dependencies.
    ///
    /// Will not cancel the previous future, but will ignore any values that it
    /// generates.
    pub fn restart(&self) {
        // self.needs_regen.set(true);
        // (self.update)();
    }

    /// Forcefully cancel a future
    pub fn cancel(&self) {
        // if let Some(task) = self.task.take() {
        //     cx.remove_future(task);
        // }
    }

    /// Get the ID of the future in Dioxus' internal scheduler
    pub fn task(&self) -> Option<Task> {
        todo!()
        // self.task.get()
    }

    /// Get the current state of the future.
    pub fn state(&self) -> UseFutureState {
        todo!()
        // match (&self.task.get(), &self.value()) {
        //     // If we have a task and an existing value, we're reloading
        //     (Some(_), Some(val)) => UseFutureState::Reloading(val),

        //     // no task, but value - we're done
        //     (None, Some(val)) => UseFutureState::Complete(val),

        //     // no task, no value - something's wrong? return pending
        //     (None, None) => UseFutureState::Pending,

        //     // Task, no value - we're still pending
        //     (Some(_), None) => UseFutureState::Pending,
        // }
    }
}

pub enum UseFutureState {
    Pending,
    Complete,
    Regenerating, // the old value
}
