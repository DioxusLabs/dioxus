use dioxus_core::{ScopeState, TaskId};
use std::cell::Cell;
use std::future::Future;

struct UseOnMount {
    needs_regen: bool,
    task: Cell<Option<TaskId>>,
}

/// A hook that runs a future when the component is mounted.
///
/// This is just [`use_effect`](crate::use_effect), but with no dependencies.
/// If you have no dependencies, it's recommended to use this, not just because it's more readable,
/// but also because it's a tiny bit more efficient.
pub fn use_on_mount<T, F>(cx: &ScopeState, future: impl FnOnce() -> F)
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let state = cx.use_hook(move || UseOnMount {
        needs_regen: true,
        task: Cell::new(None),
    });

    if state.needs_regen {
        // We don't need regen anymore
        state.needs_regen = false;
        let fut = future();

        state.task.set(Some(cx.push_future(async move {
            fut.await;
        })));
    }
}
