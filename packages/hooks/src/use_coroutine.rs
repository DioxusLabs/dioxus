use ::warnings::Warning;
use dioxus_core::prelude::{consume_context, provide_context, spawn, use_hook};
use dioxus_core::Task;
use dioxus_signals::*;
pub use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use std::future::Future;

/// Maintain a handle over a future that can be paused, resumed, and canceled.
///
/// This is an upgraded form of [`crate::use_future()`] with an integrated channel system.
/// Specifically, the coroutine generated here comes with an [`futures_channel::mpsc::UnboundedSender`]
/// built into it - saving you the hassle of building your own.
///
/// Additionally, coroutines are automatically injected as shared contexts, so
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
/// don't care about actions in your app being synchronized, you can use [`crate::use_callback()`]
/// hook to spawn multiple tasks and run them concurrently.
///
/// ### Notice
/// In order to use ``rx.next().await``, you will need to extend the ``Stream`` trait (used by ``UnboundedReceiver``)
/// by adding the ``futures-util`` crate as a dependency and adding ``StreamExt`` into scope via ``use futures_util::stream::StreamExt;``
///
/// ## Example
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// use futures_util::StreamExt;
/// enum Action {
///     Start,
///     Stop,
/// }
///
/// let chat_client = use_coroutine(|mut rx: UnboundedReceiver<Action>| async move {
///     while let Some(action) = rx.next().await {
///         match action {
///             Action::Start => {}
///             Action::Stop => {},
///         }
///     }
/// });
///
///
/// rsx! {
///     button {
///         onclick: move |_| chat_client.send(Action::Start),
///         "Start Chat Service"
///     }
/// };
/// ```
#[doc = include_str!("../docs/rules_of_hooks.md")]
pub fn use_coroutine<M, G, F>(init: G) -> Coroutine<M>
where
    M: 'static,
    G: FnOnce(UnboundedReceiver<M>) -> F,
    F: Future<Output = ()> + 'static,
{
    let mut coroutine = use_hook(|| {
        provide_context(Coroutine {
            needs_regen: Signal::new(true),
            tx: CopyValue::new(None),
            task: CopyValue::new(None),
        })
    });

    // We do this here so we can capture data with FnOnce
    // this might not be the best API
    dioxus_signals::warnings::signal_read_and_write_in_reactive_scope::allow(|| {
        dioxus_signals::warnings::signal_write_in_component_body::allow(|| {
            if *coroutine.needs_regen.peek() {
                let (tx, rx) = futures_channel::mpsc::unbounded();
                let task = spawn(init(rx));
                coroutine.tx.set(Some(tx));
                coroutine.task.set(Some(task));
                coroutine.needs_regen.set(false);
            }
        })
    });

    coroutine
}

/// Get a handle to a coroutine higher in the tree
/// Analogous to use_context_provider and use_context,
/// but used for coroutines specifically
/// See the docs for [`use_coroutine`] for more details.
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[must_use]
pub fn use_coroutine_handle<M: 'static>() -> Coroutine<M> {
    use_hook(consume_context::<Coroutine<M>>)
}

pub struct Coroutine<T: 'static> {
    needs_regen: Signal<bool>,
    tx: CopyValue<Option<UnboundedSender<T>>>,
    task: CopyValue<Option<Task>>,
}

impl<T> Coroutine<T> {
    /// Get the underlying task handle
    pub fn task(&self) -> Task {
        (*self.task.read()).unwrap()
    }

    /// Send a message to the coroutine
    pub fn send(&self, msg: T) {
        let _ = self.tx.read().as_ref().unwrap().unbounded_send(msg);
    }

    pub fn tx(&self) -> UnboundedSender<T> {
        self.tx.read().as_ref().unwrap().clone()
    }

    /// Restart this coroutine
    ///
    /// Forces the component to re-render, which will re-invoke the coroutine.
    pub fn restart(&mut self) {
        self.needs_regen.set(true);
        self.task().cancel();
    }
}

// manual impl since deriving doesn't work with generics
impl<T> Copy for Coroutine<T> {}

impl<T> Clone for Coroutine<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Coroutine<T> {
    fn eq(&self, other: &Self) -> bool {
        self.needs_regen == other.needs_regen && self.tx == other.tx && self.task == other.task
    }
}
