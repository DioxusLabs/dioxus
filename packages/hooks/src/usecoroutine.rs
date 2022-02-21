#![warn(clippy::pedantic)]

use dioxus_core::exports::bumpalo;
use dioxus_core::{LazyNodes, ScopeState, TaskId};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use std::any::Any;
use std::future::Future;
use std::{cell::Cell, rc::Rc};

/// Maintain a handle over a future that can be paused, resumed, and canceled.
///
/// This is an upgraded form of use_future with lots of bells-and-whistles.
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
pub fn use_coroutine<'a, O: 'static, M: 'static>(
    cx: &'a ScopeState,
) -> UseCoroutineBuilder<'a, O, M> {
    let inner = cx.use_hook(|_| {
        //
        UseCoroutine {
            val: Cell::new(None),
            rx: Cell::new(None),
            tx: None,
        }
    });

    UseCoroutineBuilder { cx, inner }
}

pub struct UseCoroutineBuilder<'a, O, M = ()> {
    cx: &'a ScopeState,
    inner: &'a mut UseCoroutine<O, M>,
}

impl<'a, O: 'static, M> UseCoroutineBuilder<'a, O, M> {
    // fn with_channel<I>(self) -> UseCoroutineBuilder<'a, O, I> {
    //     UseCoroutineBuilder {
    //         cx: self.cx,
    //         inner: self.inner,
    //     }
    // }

    fn build<F: Future<Output = O>>(mut self, f: impl FnOnce() -> F) -> &'a UseCoroutine<O, ()> {
        todo!()
    }
    fn build_channel<F: Future<Output = O>>(
        mut self,
        f: impl FnOnce(UnboundedReceiver<M>) -> F,
    ) -> &'a UseCoroutine<O, M> {
        todo!()
    }

    pub fn use_dep(mut self) -> Self {
        self
    }

    /// Provide the channel to downstream consumers
    pub fn provide_context(mut self) -> Self {
        self
    }

    pub fn auto_start(mut self, start: bool) -> Self {
        // if start && self.inner.run_count.get() == 1 {
        //     self.start();
        // }
        self
    }
}

pub struct UseCoroutine<O, M = ()> {
    val: Cell<Option<O>>,
    rx: Cell<Option<UnboundedReceiver<M>>>,
    tx: Option<UnboundedSender<M>>,
}

impl<O, M> UseCoroutine<O, M> {
    pub fn is_running(&self) -> bool {
        false
        // self.inner.running.get()
    }

    pub fn start(&self) {
        // if !self.is_running() {
        //     if let Some(mut fut) = self.create_fut.take() {
        //         let fut = fut();
        //         let ready_handle = self.inner.running.clone();

        //         let task = self.cx.push_future(async move {
        //             ready_handle.set(true);
        //             fut.await;
        //             ready_handle.set(false);
        //         });

        //         self.inner.task_id.set(Some(task));
        //     }
        // }
    }

    pub fn send(&self, msg: M) {
        if let Some(tx) = self.tx.clone() {
            if tx.unbounded_send(msg).is_err() {
                log::error!("Failed to send message");
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
        let poll_tasks = use_coroutine(&cx).auto_start(false).build(|| async {
            loop {
                println!("polling tasks");
            }
        });

        todo!()
    }

    fn app_with_channel(cx: Scope) -> Element {
        // let poll_tasks = use_coroutine(&cx).build_channel(|mut rx| async move {
        //     while let Some(msg) = rx.next().await {
        //         println!("polling tasks: {}", msg);
        //     }
        // });

        let poll_tasks =
            use_coroutine(&cx).build_channel(|mut rx: UnboundedReceiver<()>| async move {
                while let Some(msg) = rx.next().await {
                    println!("polling tasks: {:?}", msg);
                }
            });

        // poll_tasks.send(10);

        todo!()
    }
}

mod use2 {
    #![allow(missing_docs)]

    use dioxus_core::{ScopeState, TaskId};
    use std::{
        any::Any,
        cell::{Cell, RefCell},
        future::Future,
        rc::Rc,
    };

    /// A future that resolves to a value.
    ///
    /// This runs the future only once - though the future may be regenerated
    /// through the [`UseFuture::restart`] method.
    ///
    /// This is commonly used for components that cannot be rendered until some
    /// asynchronous operation has completed.
    ///
    ///
    ///
    ///
    ///
    pub fn use_future<'a>(
        // pub fn use_future<'a, T: 'static, F: Future<Output = T> + 'static>(
        cx: &'a ScopeState,
    ) -> &'a UseFuture<()> {
        //     let state = cx.use_hook(move |_| UseFuture {
        //         update: cx.schedule_update(),
        //         needs_regen: Cell::new(true),
        //         slot: Rc::new(Cell::new(None)),
        //         value: None,
        //         task: None,
        //         pending: true,
        //         dep_cont: Cell::new(0),
        //         deps: RefCell::new(Vec::new()),
        //         first_time: true,
        //     });

        //     if let Some(value) = state.slot.take() {
        //         state.value = Some(value);
        //         state.task = None;
        //     }

        //     if state.needs_regen.get() {
        //         // We don't need regen anymore
        //         state.needs_regen.set(false);
        //         state.pending = false;

        //         // Create the new future
        //         let fut = new_fut();

        //         // Clone in our cells
        //         let slot = state.slot.clone();
        //         let updater = state.update.clone();

        //         state.task = Some(cx.push_future(async move {
        //             let res = fut.await;
        //             slot.set(Some(res));
        //             updater();
        //         }));
        //     }

        //     state.first_time = false;

        //     state

        // new_fut: impl FnOnce() -> F,

        todo!()
    }

    pub enum FutureState<'a, T> {
        Pending,
        Complete(&'a T),
        Regenerating(&'a T), // the old value
    }

    pub struct UseFuture<T> {
        update: Rc<dyn Fn()>,
        needs_regen: Cell<bool>,
        value: Option<T>,
        pending: bool,
        slot: Rc<Cell<Option<T>>>,
        task: Option<TaskId>,
        deps: RefCell<Vec<Box<dyn Any>>>,
        dep_cont: Cell<usize>,
        first_time: bool,
    }

    impl<T> UseFuture<T> {
        pub fn restart(&self) {
            self.needs_regen.set(true);
            (self.update)();
        }

        // clears the value in the future slot without starting the future over
        pub fn clear(&self) -> Option<T> {
            (self.update)();
            self.slot.replace(None)
        }

        // Manually set the value in the future slot without starting the future over
        pub fn set(&self, new_value: T) {
            self.slot.set(Some(new_value));
            self.needs_regen.set(true);
            (self.update)();
        }

        pub fn value(&self) -> Option<&T> {
            self.value.as_ref()
        }

        pub fn state(&self) -> FutureState<T> {
            // self.value.as_ref()
            FutureState::Pending
        }

        /// Add this value to the dependency list
        ///
        /// This is a hook and should be called during the initial hook process.
        /// It should •not• be called in a conditional.
        pub fn use_dep<F: 'static + PartialEq + Clone>(&self, dependency: &F) -> &Self {
            let count = self.dep_cont.get();
            let mut deps = self.deps.borrow_mut();

            match deps.get_mut(count) {
                Some(dep) => match dep.downcast_mut::<F>() {
                    Some(saved_dep) => {
                        if dependency != saved_dep {
                            *saved_dep = dependency.clone();
                            self.needs_regen.set(true);
                        }
                    }
                    None => {
                        if cfg!(debug_assertions) {
                            panic!("Tried to use a dependency for use_future outside of the use_future hook.");
                        }
                    }
                },
                None => deps.push(Box::new(dependency.to_owned())),
            }

            self
        }

        pub fn restart_if(&self, f: impl FnOnce() -> bool) -> &Self {
            self
        }

        pub fn build<F>(&self, new_fut: impl FnOnce() -> F) {}
    }

    #[test]
    fn test_use_future_deps() {
        use dioxus_core::prelude::*;

        struct MyProps {
            val: String,
            name: i32,
        }

        fn app(cx: Scope<MyProps>) -> Element {
            let MyProps { val, name } = cx.props;

            // let val = use_c(&cx)
            //     .use_dep(val)
            //     .restart_if(|| false)
            //     .use_dep(name)
            //     .build(|(val, name)| async move {});

            // async fn fetch_thing(name: String, num: i32) -> String {
            //     format!("{} {}", name, num)
            // }

            // let val = use_future(&cx, || fetch_thing(val.clone(), *name))
            //     .with_dep(val)
            //     .with_dep(name)
            //     .restart_if(|| false);

            None
        }
    }

    // pub struct CoroutineBuilder<'a, const PARAM: usize> {
    //     deps: Vec<Box<dyn Any>>,
    //     cx: &'a ScopeState,
    // }

    // macro_rules! dep_impl {
    //     ($id1:literal to $id2:literal) => {
    //         impl<'a> CoroutineBuilder<'a, $id1> {
    //             pub fn use_dep<F: 'static + PartialEq + Clone>(
    //                 mut self,
    //                 dep: &F,
    //             ) -> CoroutineBuilder<'a, $id2> {
    //                 self.deps.push(Box::new(dep.clone()));
    //                 unsafe { std::mem::transmute(self) }
    //             }
    //         }
    //     };
    // }

    // dep_impl!(0 to 1);
    // dep_impl!(1 to 2);
    // dep_impl!(2 to 3);
    // dep_impl!(3 to 4);
    // dep_impl!(4 to 5);
    // dep_impl!(5 to 6);
    // dep_impl!(6 to 7);
    // dep_impl!(7 to 8);
    // dep_impl!(8 to 9);
}
