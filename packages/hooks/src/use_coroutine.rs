use dioxus_core::{ScopeState, TaskId};
pub use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use std::future::Future;

/// Maintain a handle over a future that can be paused, resumed, and canceled.
///
/// This is an upgraded form of [`use_future`] with an integrated channel system.
/// Specifically, the coroutine generated here comes with an [`UnboundedChannel`]
/// built into it - saving you the hassle of building your own.
///
/// Addititionally, coroutines are automatically injected as shared contexts, so
/// downstream components can tap into a coroutine's channel and send messages
/// into a singular async event loop.
///
/// This makes it effective for apps that need to interact with an event loop or
/// some asynchronous code without thinking too hard about state.
///
/// ## Global State
///
/// Typically, writing apps that handle concurrency properly can be difficult,
/// so the intention of this hook is to make it easy to join and poll async tasks
/// concurrently in a centralized place. You'll find that you can have much better
/// control over your app's state if you centralize your async actions, even under
/// the same concurrent context. This makes it easier to prevent undeseriable
/// states in your UI while various async tasks are already running.
///
/// This hook is especially powerful when combined with Fermi. We can store important
/// global data in a coroutine, and then access display-level values from the rest
/// of our app through atoms.
///
/// ## UseCallback instead
///
/// However, you must plan out your own concurrency and synchronization. If you
/// don't care about actions in your app being synchronized, you can use [`use_callback`]
/// hook to spawn multiple tasks and run them concurrently.
///
/// ### Notice
/// In order to use ``rx.next().await``, you will need to extend the ``Stream`` trait (used by ``UnboundedReceiver``)
/// by adding the ``futures-util`` crate as a dependency and adding ``StreamExt`` into scope via ``use futures_util::stream::StreamExt;``
///
/// ## Example
///
/// ```rust, ignore
/// enum Action {
///     Start,
///     Stop,
/// }
///
/// let chat_client = use_coroutine(cx, |mut rx: UnboundedReceiver<Action>| async move {
///     while let Some(action) = rx.next().await {
///         match action {
///             Action::Start => {}
///             Action::Stop => {},
///         }
///     }
/// });
///
///
/// cx.render(rsx!{
///     button {
///         onclick: move |_| chat_client.send(Action::Start),
///         "Start Chat Service"
///     }
/// })
/// ```
pub fn use_coroutine<M, G, F>(cx: &ScopeState, init: G) -> &Coroutine<M>
where
    M: 'static,
    G: FnOnce(UnboundedReceiver<M>) -> F,
    F: Future<Output = ()> + 'static,
{
    cx.use_hook(|| {
        let (tx, rx) = futures_channel::mpsc::unbounded();
        let task = cx.push_future(init(rx));
        cx.provide_context(Coroutine { tx, task })
    })
}

/// Get a handle to a coroutine higher in the tree
///
/// See the docs for [`use_coroutine`] for more details.
#[must_use]
pub fn use_coroutine_handle<M: 'static>(cx: &ScopeState) -> Option<&Coroutine<M>> {
    cx.use_hook(|| cx.consume_context::<Coroutine<M>>())
        .as_ref()
}

pub struct Coroutine<T> {
    tx: UnboundedSender<T>,
    task: TaskId,
}

// for use in futures
impl<T> Clone for Coroutine<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            task: self.task,
        }
    }
}

impl<T> Coroutine<T> {
    /// Get the ID of this coroutine
    #[must_use]
    pub fn task_id(&self) -> TaskId {
        self.task
    }

    /// Send a message to the coroutine
    pub fn send(&self, msg: T) {
        let _ = self.tx.unbounded_send(msg);
    }
}

impl<T> PartialEq for Coroutine<T> {
    fn eq(&self, other: &Self) -> bool {
        self.task == other.task
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused)]

    use super::*;
    use dioxus_core::prelude::*;
    use futures_channel::mpsc::unbounded;
    use futures_util::StreamExt;

    fn app(cx: Scope, name: String) -> Element {
        let task = use_coroutine(cx, |mut rx: UnboundedReceiver<i32>| async move {
            while let Some(msg) = rx.next().await {
                println!("got message: {msg}");
            }
        });

        let task2 = use_coroutine(cx, view_task);

        let task3 = use_coroutine(cx, |rx| complex_task(rx, 10));

        todo!()
    }

    async fn view_task(mut rx: UnboundedReceiver<i32>) {
        while let Some(msg) = rx.next().await {
            println!("got message: {msg}");
        }
    }

    enum Actions {
        CloseAll,
        OpenAll,
    }

    async fn complex_task(mut rx: UnboundedReceiver<Actions>, name: i32) {
        while let Some(msg) = rx.next().await {
            match msg {
                Actions::CloseAll => todo!(),
                Actions::OpenAll => todo!(),
            }
        }
    }
}
