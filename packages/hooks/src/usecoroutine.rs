use dioxus_core::{AnyContext, Scope};
use futures::Future;
use std::{
    cell::{Cell, RefCell},
    pin::Pin,
    rc::Rc,
};

pub fn use_coroutine<'a, F: Future<Output = ()> + 'static>(
    cx: &dyn AnyContext<'a>,
    mut f: impl FnMut() -> F + 'a,
) -> CoroutineHandle<'a> {
    let cx = cx.get_scope();
    cx.use_hook(
        move |_| State {
            running: Default::default(),
            fut: Default::default(),
            submit: Default::default(),
        },
        |state| {
            let fut_slot = state.fut.clone();
            let running = state.running.clone();
            let submit: Box<dyn FnOnce() + 'a> = Box::new(move || {
                let g = async move {
                    running.set(true);
                    f().await;
                    running.set(false);
                };
                let p: Pin<Box<dyn Future<Output = ()>>> = Box::pin(g);
                fut_slot
                    .borrow_mut()
                    .replace(unsafe { std::mem::transmute(p) });
            });

            let submit = unsafe { std::mem::transmute(submit) };
            state.submit.get_mut().replace(submit);

            if state.running.get() {
                let mut fut = state.fut.borrow_mut();
                cx.push_task(|| fut.as_mut().unwrap().as_mut());
            } else {
                // make sure to drop the old future
                if let Some(fut) = state.fut.borrow_mut().take() {
                    drop(fut);
                }
            }
            CoroutineHandle { cx, inner: state }
        },
    )
}

struct State {
    running: Rc<Cell<bool>>,
    submit: RefCell<Option<Box<dyn FnOnce()>>>,
    fut: Rc<RefCell<Option<Pin<Box<dyn Future<Output = ()>>>>>>,
}

pub struct CoroutineHandle<'a> {
    cx: &'a Scope,
    inner: &'a State,
}
impl Clone for CoroutineHandle<'_> {
    fn clone(&self) -> Self {
        CoroutineHandle {
            cx: self.cx,
            inner: self.inner,
        }
    }
}
impl Copy for CoroutineHandle<'_> {}

impl<'a> CoroutineHandle<'a> {
    pub fn start(&self) {
        if self.is_running() {
            return;
        }
        if let Some(submit) = self.inner.submit.borrow_mut().take() {
            submit();
            let mut fut = self.inner.fut.borrow_mut();
            self.cx.push_task(|| fut.as_mut().unwrap().as_mut());
        }
    }

    pub fn is_running(&self) -> bool {
        self.inner.running.get()
    }

    pub fn resume(&self) {}
}
