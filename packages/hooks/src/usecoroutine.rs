#![warn(clippy::pedantic)]

use dioxus_core::exports::bumpalo;
use dioxus_core::{LazyNodes, ScopeState, TaskId};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use std::any::Any;
use std::future::Future;
use std::{cell::Cell, rc::Rc};

/// Maintain a handle over a future that can be paused, resumed, and canceled.
///
/// This is an upgraded form of [`use_future`] with lots of bells-and-whistles.
///
/// [`use_coroutine`] is well suited for long-running tasks and is very customizable.
///
///
/// ## Long running tasks
///
///
///
/// ## One-off tasks
///
///
/// ## Cancellation
///
///
/// ## Global State
#[allow(clippy::mut_from_ref)]
pub fn use_coroutine<O: 'static>(cx: &ScopeState) -> &mut UseCoroutine<O, ()> {
    cx.use_hook(|_| {
        //
        UseCoroutine {
            val: Cell::new(None),
            rx: Cell::new(None),
            tx: None,
            first_run: true,
            deps: vec![],
            dep_cnt: 0,
            needs_regen: false,
            auto_start: true,
        }
    })
}

pub struct UseCoroutine<O, M = ()> {
    val: Cell<Option<O>>,
    rx: Cell<Option<UnboundedReceiver<M>>>,
    tx: Option<UnboundedSender<M>>,
    first_run: bool,
    deps: Vec<Box<dyn Any>>,
    dep_cnt: usize,
    needs_regen: bool,
    auto_start: bool,
}

pub enum FutureState<'a, T> {
    Pending,
    Complete(&'a T),
    Regenerating(&'a T), // the old value
}

impl<O> UseCoroutine<O, ()> {
    /// explicitly set the type of the channel used by the coroutine
    fn with_channel<S>(&mut self) -> &mut UseCoroutine<O, S> {
        if self.first_run {
            // self.provide_context()
        }
        todo!()
    }

    /// explicitly set the type of the channel used by the coroutine
    fn with_channel_isolate<S>(&mut self) -> &mut UseCoroutine<O, S> {
        todo!()
    }
}

impl<O, M> UseCoroutine<O, M> {
    pub fn is_running(&self) -> bool {
        false
        // self.running.get()
    }

    pub fn start(&self) {
        // if !self.is_running() {
        //     if let Some(mut fut) = self.create_fut.take() {
        //         let fut = fut();
        //         let ready_handle = self.running.clone();

        //         let task = self.cx.push_future(async move {
        //             ready_handle.set(true);
        //             fut.await;
        //             ready_handle.set(false);
        //         });

        //         self.task_id.set(Some(task));
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

    fn build<F: Future<Output = O>>(&mut self, f: impl FnOnce(UnboundedReceiver<M>) -> F) -> &Self {
        self.first_run = false;
        if self.auto_start || self.needs_regen {
            //
        }

        self
    }

    pub fn auto_start(mut self, start: bool) -> Self {
        // if start && self.run_count.get() == 1 {
        //     self.start();
        // }
        self
    }

    /// Add this value to the dependency list
    ///
    /// This is a hook and should be called during the initial hook process.
    /// It should •not• be called in a conditional.
    pub fn with_dep<F: 'static + PartialEq + Clone>(&mut self, dependency: &F) -> &mut Self {
        if let Some(dep) = self.deps.get_mut(self.dep_cnt) {
            if let Some(saved_dep) = dep.downcast_mut::<F>() {
                if dependency != saved_dep {
                    *saved_dep = dependency.clone();
                    self.needs_regen = true;
                }
            };
        } else {
            self.deps.push(Box::new(dependency.to_owned()));
            self.needs_regen = true;
        }

        self
    }

    pub fn restart_if(&self, f: impl FnOnce() -> bool) -> &Self {
        self
    }

    // pub fn resume(&self) {}
    // pub fn stop(&self) {}
    // pub fn restart(&self) {}
}

pub struct CoroutineContext<T> {
    tx: UnboundedSender<T>,
}

#[cfg(test)]
mod tests {
    #![allow(unused)]

    use super::*;
    use dioxus_core::exports::futures_channel::mpsc::unbounded;
    use dioxus_core::prelude::*;
    use futures_util::StreamExt;

    fn app(cx: Scope, name: String) -> Element {
        let task = use_coroutine(&cx)
            .with_dep(&name)
            .with_channel::<i32>()
            .build(|mut rx| async move {
                while let Some(msg) = rx.next().await {
                    println!("got message: {}", msg);
                }
            });

        None
    }
}
