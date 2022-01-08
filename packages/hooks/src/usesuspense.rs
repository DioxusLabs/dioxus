use std::{cell::Cell, future::Future, rc::Rc};

use dioxus_core::{Element, ScopeState, TaskId};

pub fn use_suspense<R: 'static, F: Future<Output = R> + 'static>(
    cx: &ScopeState,
    create_future: impl FnOnce() -> F,
    render: impl FnOnce(&R) -> Element,
) -> Element {
    let sus = cx.use_hook(|_| {
        let fut = create_future();

        let wip_value: Rc<Cell<Option<R>>> = Default::default();

        let wip = wip_value.clone();
        let new_fut = async move {
            let val = fut.await;
            wip.set(Some(val));
        };

        let task = cx.push_future(new_fut);
        SuspenseInner {
            _task: task,
            value: None,
            _wip_value: wip_value,
        }
    });

    if let Some(value) = sus.value.as_ref() {
        render(value)
    } else {
        // generate a placeholder node if the future isnt ready
        None
    }
}

struct SuspenseInner<R> {
    _task: TaskId,
    _wip_value: Rc<Cell<Option<R>>>,
    value: Option<R>,
}
