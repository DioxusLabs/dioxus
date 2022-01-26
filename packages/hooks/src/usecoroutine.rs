#![warn(clippy::pedantic)]

use bumpalo::boxed::Box as BumpBox;
use dioxus_core::exports::bumpalo;
use dioxus_core::{LazyNodes, ScopeState, TaskId};
use std::future::Future;
use std::{cell::Cell, rc::Rc};

/// Maintain a handle over a future that can be paused, resumed, and canceled.
///
///
///
///
///
///
///
///
///
///
///
///
///
///
///
///
pub fn use_coroutine<'a, F>(
    cx: &'a ScopeState,
    create_future: impl FnOnce() -> F + 'a,
) -> UseCoroutine<'a, F>
where
    F: Future<Output = ()> + 'static,
{
    let state = cx.use_hook(move |_| CoroutineInner {
        task_id: Cell::new(None),
        running: std::rc::Rc::default(),
        run_count: Cell::new(0),
    });

    // as an optimization, we use the bump arena to allocate the callback instead of boxes
    // that way we don't always call the constructor, but it's still efficient
    // safety: bumpalo is limited in constructing unsized box types, so we have to do it through dynamic dispatch
    let boxed: BumpBox<'a, dyn FnMut() -> F + 'a> = unsafe {
        let mut bump = None;
        cx.render(LazyNodes::new(move |f| {
            bump.replace(f.bump());
            f.static_text("")
        }));
        let mut slot = Some(create_future);
        let bump = bump.expect("bump is assigned during render");
        BumpBox::from_raw(bump.alloc(move || {
            let inner = slot.take().expect("closure to not be called twice");
            inner()
        }))
    };

    state.run_count.set(state.run_count.get() + 1);

    UseCoroutine {
        inner: state,
        create_fut: Cell::new(Some(boxed)),
        cx,
    }
}

struct CoroutineInner {
    running: Rc<Cell<bool>>,
    task_id: Cell<Option<TaskId>>,
    run_count: Cell<u32>,
}

pub struct UseCoroutine<'a, F: Future<Output = ()> + 'static> {
    create_fut: Cell<Option<BumpBox<'a, dyn FnMut() -> F + 'a>>>,
    inner: &'a CoroutineInner,
    cx: &'a ScopeState,
}

impl<'a, F: Future<Output = ()> + 'static> UseCoroutine<'a, F> {
    pub fn auto_start(&self, start: bool) -> &Self {
        if start && self.inner.run_count.get() == 1 {
            self.start();
        }
        self
    }

    pub fn is_running(&self) -> bool {
        self.inner.running.get()
    }

    pub fn start(&self) {
        if !self.is_running() {
            if let Some(mut fut) = self.create_fut.take() {
                let fut = fut();
                let ready_handle = self.inner.running.clone();

                let task = self.cx.push_future(async move {
                    ready_handle.set(true);
                    fut.await;
                    ready_handle.set(false);
                });

                self.inner.task_id.set(Some(task));
            }
        }
    }

    // todo: wire these up, either into the task system or into the coroutine system itself
    // we would have change how we poll the coroutine and how its awaken

    // pub fn resume(&self) {}
    // pub fn stop(&self) {}
    // pub fn restart(&self) {}
}

#[cfg(test)]
mod tests {
    #![allow(unused)]

    use super::*;
    use dioxus_core::exports::futures_channel::mpsc::unbounded;
    use dioxus_core::prelude::*;
    use futures_util::StreamExt;

    fn app(cx: Scope) -> Element {
        let poll_tasks = use_coroutine(&cx, || async {
            loop {
                println!("polling tasks");
            }
        });

        poll_tasks.auto_start(true);

        todo!()
    }

    fn app_with_channel(cx: Scope) -> Element {
        let (tx, mut rx) = unbounded();

        let tx = cx.use_hook(|_| tx);

        let poll_tasks = use_coroutine(&cx, move || async move {
            while let Some(msg) = rx.next().await {
                println!("polling tasks: {}", msg);
            }
        });

        poll_tasks.auto_start(true);

        tx.unbounded_send("asd").unwrap();

        todo!()
    }
}
