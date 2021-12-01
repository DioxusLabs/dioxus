use dioxus_core::Context;
use futures::Future;
use std::{
    cell::{Cell, RefCell},
    pin::Pin,
    rc::Rc,
};

pub fn use_coroutine<'a, F: Future<Output = ()> + 'a>(
    cx: Context<'a>,
    f: impl FnOnce() -> F + 'a,
) -> CoroutineHandle {
    //
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
    cx: Context<'a>,
    inner: &'a State,
}

impl<'a> CoroutineHandle<'a> {
    pub fn start(&self) {
        if self.inner.running.get() {
            return;
        }
        if let Some(submit) = self.inner.submit.borrow_mut().take() {
            submit();
            let mut fut = self.inner.fut.borrow_mut();
            self.cx.push_task(|| fut.as_mut().unwrap().as_mut());
        }
    }
    pub fn resume(&self) {}
}
