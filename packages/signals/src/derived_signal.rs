use dioxus_core::{current_scope_id, spawn_isomorphic, ReactiveContext};
use futures_util::StreamExt;

use crate::{Signal, WritableExt};

pub(crate) fn derived_signal<T: 'static>(
    mut init: impl FnMut() -> T + 'static,
    location: &'static std::panic::Location<'static>,
) -> Signal<T> {
    let (tx, mut rx) = futures_channel::mpsc::unbounded();

    let rc = ReactiveContext::new_with_callback(
        move || _ = tx.unbounded_send(()),
        current_scope_id(),
        location,
    );

    // Create a new signal in that context, wiring up its dependencies and subscribers
    let mut recompute = move || rc.reset_and_run_in(&mut init);
    let value = recompute();
    let mut state: Signal<T> = Signal::new_with_caller(value, location);

    spawn_isomorphic(async move {
        while rx.next().await.is_some() {
            // Remove any pending updates
            while rx.try_next().is_ok() {}
            state.set(recompute());
        }
    });

    state
}
