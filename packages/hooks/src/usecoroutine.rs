use bumpalo::boxed::Box as BumpBox;
use dioxus_core::exports::bumpalo;
use dioxus_core::exports::futures_channel;
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
) -> UseCoroutine<F>
where
    F: Future<Output = ()> + 'static,
{
    let state = cx.use_hook(move |_| CoroutineInner {
        _id: None,
        running: Default::default(),
    });

    let mut bump = None;

    // as an optimization, we use the bump arena to allocate the callback instead of boxes
    // that way we don't always call the constructor, but it's still efficient
    cx.render(LazyNodes::new(move |f| {
        bump.replace(f.bump());
        f.static_text("")
    }));

    let mut slot = Some(create_future);

    // safety: bumpalo is limited in constructing unsized box types, so we have to do it through dynamic dispatch
    let boxed: BumpBox<'a, dyn FnMut() -> F + 'a> = unsafe {
        BumpBox::from_raw(bump.unwrap().alloc(move || {
            let inner = slot.take().unwrap();
            inner()
        }))
    };

    UseCoroutine {
        inner: state,
        create_fut: Cell::new(Some(boxed)),
        cx,
    }
}

struct CoroutineInner {
    running: Rc<Cell<bool>>,
    _id: Option<TaskId>,
}

pub struct UseCoroutine<'a, F: Future<Output = ()> + 'static> {
    create_fut: Cell<Option<BumpBox<'a, dyn FnMut() -> F + 'a>>>,
    inner: &'a CoroutineInner,
    cx: &'a ScopeState,
}

impl<'a, F: Future<Output = ()> + 'static> UseCoroutine<'a, F> {
    pub fn auto_start(&self, start: bool) -> &Self {
        todo!()
    }

    pub fn start(&self) {
        if !self.is_running() {
            if let Some(mut fut) = self.create_fut.take() {
                let fut = fut();
                self.cx.push_future(fut);
            }
        }
    }

    pub fn is_running(&self) -> bool {
        // self.inner.running.get()
        false
    }

    pub fn resume(&self) {
        // self.cx.push_task(fut)
    }

    pub fn stop(&self) {}

    pub fn restart(&self) {}
}

#[test]
fn it_works() {
    use dioxus_core::prelude::*;
    fn app(cx: Scope) -> Element {
        let poll_tasks = use_coroutine(&cx, || async {
            loop {
                println!("polling tasks");
            }
        });

        poll_tasks.auto_start(true);

        todo!()
    }
}
