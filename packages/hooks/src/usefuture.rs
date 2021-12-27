use dioxus_core::{ScopeState, TaskId};
use std::{cell::Cell, future::Future, rc::Rc};

pub fn use_future<'a, T: 'static, F: Future<Output = T> + 'static>(
    cx: &'a ScopeState,
    f: impl FnOnce() -> F,
) -> (Option<&T>, FutureHandle<'a, T>) {
    cx.use_hook(
        |_| {
            //
            let fut = f();
            let slot = Rc::new(Cell::new(None));
            let updater = cx.schedule_update();

            let _slot = slot.clone();
            let new_fut = async move {
                let res = fut.await;
                _slot.set(Some(res));
                updater();
            };
            let task = cx.push_future(new_fut);

            UseFutureInner {
                needs_regen: true,
                slot,
                value: None,
                task: Some(task),
            }
        },
        |state| {
            if let Some(value) = state.slot.take() {
                state.value = Some(value);
                state.task = None;
            }
            (
                state.value.as_ref(),
                FutureHandle {
                    cx,
                    value: Cell::new(None),
                },
            )
        },
    )
}

struct UseFutureInner<T> {
    needs_regen: bool,
    value: Option<T>,
    slot: Rc<Cell<Option<T>>>,
    task: Option<TaskId>,
}

pub struct FutureHandle<'a, T> {
    cx: &'a ScopeState,
    value: Cell<Option<T>>,
}
