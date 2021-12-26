use dioxus_core::{ScopeState, TaskId};
use std::{cell::Cell, future::Future};

pub fn use_future<'a, T: 'static, F: Future<Output = T>>(
    cx: &'a ScopeState,
    f: impl FnOnce() -> F,
) -> FutureHandle<'a, T> {
    cx.use_hook(
        |_| {
            //
            UseFutureInner {
                needs_regen: true,
                task: None,
            }
        },
        |_| {
            //
            FutureHandle {
                cx,
                value: Cell::new(None),
            }
        },
    )
}

struct UseFutureInner {
    needs_regen: bool,
    task: Option<TaskId>,
}

pub struct FutureHandle<'a, T> {
    cx: &'a ScopeState,
    value: Cell<Option<T>>,
}
