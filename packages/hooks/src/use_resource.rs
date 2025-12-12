#![allow(missing_docs)]

use crate::{use_callback, use_signal, use_waker, UseWaker};

use dioxus_core::{
    spawn, use_hook, Callback, IntoAttributeValue, IntoDynNode, ReactiveContext, RenderError,
    Subscribers, SuspendedFuture, Task,
};
use dioxus_signals::*;
use futures_util::{
    future::{self},
    pin_mut, FutureExt, StreamExt,
};
use std::{cell::Cell, future::Future, rc::Rc};
use std::{fmt::Debug, ops::Deref};

#[doc = include_str!("../docs/use_resource.md")]
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[doc = include_str!("../docs/moving_state_around.md")]
#[doc(alias = "use_async_memo")]
#[doc(alias = "use_memo_async")]
#[track_caller]
pub fn use_resource<T, F>(mut future: impl FnMut() -> F + 'static) -> Resource<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let location = std::panic::Location::caller();

    let mut value = use_signal(|| None);
    let mut state = use_signal(|| UseResourceState::Pending);
    let (rc, changed) = use_hook(|| {
        let (rc, changed) = ReactiveContext::new_with_origin(location);
        (rc, Rc::new(Cell::new(Some(changed))))
    });

    let mut waker = use_waker::<()>();

    let cb = use_callback(move |_| {
        // Set the state to Pending when the task is restarted
        state.set(UseResourceState::Pending);

        // Create the user's task
        let fut = rc.reset_and_run_in(&mut future);

        // Spawn a wrapper task that polls the inner future and watches its dependencies
        spawn(async move {
            // Move the future here and pin it so we can poll it
            let fut = fut;
            pin_mut!(fut);

            // Run each poll in the context of the reactive scope
            // This ensures the scope is properly subscribed to the future's dependencies
            let res = future::poll_fn(|cx| {
                rc.run_in(|| {
                    tracing::trace_span!("polling resource", location = %location)
                        .in_scope(|| fut.poll_unpin(cx))
                })
            })
            .await;

            // Set the value and state
            state.set(UseResourceState::Ready);
            value.set(Some(res));

            // Notify that the value has changed
            waker.wake(());
        })
    });

    let mut task = use_hook(|| Signal::new(cb(())));

    use_hook(|| {
        let mut changed = changed.take().unwrap();
        spawn(async move {
            loop {
                // Wait for the dependencies to change
                let _ = changed.next().await;

                // Stop the old task
                task.write().cancel();

                // Start a new task
                task.set(cb(()));
            }
        })
    });

    Resource {
        task,
        value,
        state,
        waker,
        callback: cb,
    }
}

/// A handle to a reactive future spawned with [`use_resource`] that can be used to modify or read the result of the future.
///
/// ## Example
///
/// Reading the result of a resource:
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use std::time::Duration;
/// fn App() -> Element {
///     let mut revision = use_signal(|| "1d03b42");
///     let mut resource = use_resource(move || async move {
///         // This will run every time the revision signal changes because we read the count inside the future
///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
///     });
///
///     // Since our resource may not be ready yet, the value is an Option. Our request may also fail, so the get function returns a Result
///     // The complete type we need to match is `Option<Result<String, reqwest::Error>>`
///     // We can use `read_unchecked` to keep our matching code in one statement while avoiding a temporary variable error (this is still completely safe because dioxus checks the borrows at runtime)
///     match &*resource.read_unchecked() {
///         Some(Ok(value)) => rsx! { "{value:?}" },
///         Some(Err(err)) => rsx! { "Error: {err}" },
///         None => rsx! { "Loading..." },
///     }
/// }
/// ```
#[derive(Debug)]
pub struct Resource<T: 'static> {
    waker: UseWaker<()>,
    value: Signal<Option<T>>,
    task: Signal<Task>,
    state: Signal<UseResourceState>,
    callback: Callback<(), Task>,
}

impl<T> PartialEq for Resource<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.state == other.state
            && self.task == other.task
            && self.callback == other.callback
    }
}

impl<T> Clone for Resource<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Resource<T> {}

/// A signal that represents the state of the resource
// we might add more states (panicked, etc)
#[derive(Clone, Copy, PartialEq, Hash, Eq, Debug)]
pub enum UseResourceState {
    /// The resource's future is still running
    Pending,

    /// The resource's future has been forcefully stopped
    Stopped,

    /// The resource's future has been paused, tempoarily
    Paused,

    /// The resource's future has completed
    Ready,
}

impl<T> Resource<T> {
    /// Restart the resource's future.
    ///
    /// This will cancel the current future and start a new one.
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     rsx! {
    ///         button {
    ///             // We can get a signal with the value of the resource with the `value` method
    ///             onclick: move |_| resource.restart(),
    ///             "Restart resource"
    ///         }
    ///         "{resource:?}"
    ///     }
    /// }
    /// ```
    pub fn restart(&mut self) {
        self.task.write().cancel();
        let new_task = self.callback.call(());
        self.task.set(new_task);
    }

    /// Forcefully cancel the resource's future.
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     rsx! {
    ///         button {
    ///             // We can cancel the resource before it finishes with the `cancel` method
    ///             onclick: move |_| resource.cancel(),
    ///             "Cancel resource"
    ///         }
    ///         "{resource:?}"
    ///     }
    /// }
    /// ```
    pub fn cancel(&mut self) {
        self.state.set(UseResourceState::Stopped);
        self.task.write().cancel();
    }

    /// Pause the resource's future.
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     rsx! {
    ///         button {
    ///             // We can pause the future with the `pause` method
    ///             onclick: move |_| resource.pause(),
    ///             "Pause"
    ///         }
    ///         button {
    ///             // And resume it with the `resume` method
    ///             onclick: move |_| resource.resume(),
    ///             "Resume"
    ///         }
    ///         "{resource:?}"
    ///     }
    /// }
    /// ```
    pub fn pause(&mut self) {
        self.state.set(UseResourceState::Paused);
        self.task.write().pause();
    }

    /// Resume the resource's future.
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     rsx! {
    ///         button {
    ///             // We can pause the future with the `pause` method
    ///             onclick: move |_| resource.pause(),
    ///             "Pause"
    ///         }
    ///         button {
    ///             // And resume it with the `resume` method
    ///             onclick: move |_| resource.resume(),
    ///             "Resume"
    ///         }
    ///         "{resource:?}"
    ///     }
    /// }
    /// ```
    pub fn resume(&mut self) {
        if self.finished() {
            return;
        }

        self.state.set(UseResourceState::Pending);
        self.task.write().resume();
    }

    /// Clear the resource's value. This will just reset the value. It will not modify any running tasks.
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     rsx! {
    ///         button {
    ///             // We clear the value without modifying any running tasks with the `clear` method
    ///             onclick: move |_| resource.clear(),
    ///             "Clear"
    ///         }
    ///         "{resource:?}"
    ///     }
    /// }
    /// ```
    pub fn clear(&mut self) {
        self.value.write().take();
    }

    /// Get a handle to the inner task backing this resource
    /// Modify the task through this handle will cause inconsistent state
    pub fn task(&self) -> Task {
        self.task.cloned()
    }

    /// Is the resource's future currently running?
    pub fn pending(&self) -> bool {
        matches!(*self.state.peek(), UseResourceState::Pending)
    }

    /// Is the resource's future currently finished running?
    ///
    /// Reading this does not subscribe to the future's state
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     // We can use the `finished` method to check if the future is finished
    ///     if resource.finished() {
    ///         rsx! {
    ///             "The resource is finished"
    ///         }
    ///     } else {
    ///         rsx! {
    ///             "The resource is still running"
    ///         }
    ///     }
    /// }
    /// ```
    pub fn finished(&self) -> bool {
        matches!(
            *self.state.peek(),
            UseResourceState::Ready | UseResourceState::Stopped
        )
    }

    /// Get the current state of the resource's future. This method returns a [`ReadSignal`] which can be read to get the current state of the resource or passed to other hooks and components.
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     // We can read the current state of the future with the `state` method
    ///     match resource.state().cloned() {
    ///         UseResourceState::Pending => rsx! {
    ///             "The resource is still pending"
    ///         },
    ///         UseResourceState::Paused => rsx! {
    ///             "The resource has been paused"
    ///         },
    ///         UseResourceState::Stopped => rsx! {
    ///             "The resource has been stopped"
    ///         },
    ///         UseResourceState::Ready => rsx! {
    ///             "The resource is ready!"
    ///         },
    ///     }
    /// }
    /// ```
    pub fn state(&self) -> ReadSignal<UseResourceState> {
        self.state.into()
    }

    /// Get the current value of the resource's future.  This method returns a [`ReadSignal`] which can be read to get the current value of the resource or passed to other hooks and components.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     // We can get a signal with the value of the resource with the `value` method
    ///     let value = resource.value();
    ///
    ///     // Since our resource may not be ready yet, the value is an Option. Our request may also fail, so the get function returns a Result
    ///     // The complete type we need to match is `Option<Result<String, reqwest::Error>>`
    ///     // We can use `read_unchecked` to keep our matching code in one statement while avoiding a temporary variable error (this is still completely safe because dioxus checks the borrows at runtime)
    ///     match &*value.read_unchecked() {
    ///         Some(Ok(value)) => rsx! { "{value:?}" },
    ///         Some(Err(err)) => rsx! { "Error: {err}" },
    ///         None => rsx! { "Loading..." },
    ///     }
    /// }
    /// ```
    pub fn value(&self) -> ReadSignal<Option<T>> {
        self.value.into()
    }

    /// Suspend the resource's future and only continue rendering when the future is ready
    pub fn suspend(&self) -> std::result::Result<MappedSignal<T, Signal<Option<T>>>, RenderError> {
        match self.state.cloned() {
            UseResourceState::Stopped | UseResourceState::Paused | UseResourceState::Pending => {
                let task = self.task();
                if task.paused() {
                    Ok(self.value.map(|v| v.as_ref().unwrap()))
                } else {
                    Err(RenderError::Suspended(SuspendedFuture::new(task)))
                }
            }
            _ => Ok(self.value.map(|v| v.as_ref().unwrap())),
        }
    }

    /// Asynchronously wait for the resource to be ready and read its value, while safely managing guards across the await point.
    ///
    /// This method solves a common problem when chaining resources: holding guards (like write guards from signals)
    /// across await points can cause deadlocks or borrowing issues. `read_async()` accepts these guards as parameters,
    /// drops them temporarily while waiting for the resource, and returns them back along with the resource value.
    ///
    /// ## How it works
    ///
    /// 1. **Accepts values**: Pass guards or other values as a tuple parameter (or `()` for no values)
    /// 2. **Waits safely**: If the resource isn't ready, drops the guards to prevent deadlocks while waiting
    /// 3. **Returns together**: Once ready, returns the resource value and your guards as a tuple
    /// 4. **Compiler reminder**: Forces explicit acknowledgment that you're managing guards across an await
    ///
    /// ## Example
    ///
    /// Chaining two resources while maintaining write guards:
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// fn app() -> Element {
    ///     let mut result = use_signal(|| "Waiting...".to_string());
    ///     
    ///     // First resource fetches data
    ///     let mut data = use_resource(move || async move {
    ///         reqwest::get("https://api.example.com/data")
    ///             .await?
    ///             .json::<String>()
    ///             .await
    ///     });
    ///     
    ///     // Second resource chains off the first using read_async
    ///     let _process = use_resource(move || async move {
    ///         // Get a write guard that we need to hold across the await
    ///         let result_write = result.write();
    ///         
    ///         // Wait for the first resource and get our guard back
    ///         let (data_ref, mut result_write) = data.read_async((result_write,)).await;
    ///         
    ///         // Now we can safely use both the data and our guard
    ///         match &*data_ref {
    ///             Ok(value) => *result_write = value.clone(),
    ///             Err(e) => *result_write = format!("Error: {}", e),
    ///         }
    ///         
    ///         Ok::<(), anyhow::Error>(())
    ///     });
    ///     
    ///     rsx! { "{result}" }
    /// }
    /// ```
    ///
    /// If you don't need to hold any guards, pass `()`:
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # let data = use_resource(move || async move { Ok::<String, ()>("test".to_string()) });
    /// # async {
    /// // Just wait for the resource without any guards
    /// let data_ref = data.read_async(()).await;
    /// # };
    /// ```
    pub async fn read_async<'a, M>(&'a self, maybe_drop: M) -> M::Out
    where
        M: MaybeDrop<generational_box::GenerationalRef<std::cell::Ref<'a, T>>>,
    {
        let read: generational_box::GenerationalRef<std::cell::Ref<'a, Option<T>>> = self.read();
        if read.is_none() {
            drop(read);
            drop(maybe_drop);
            let _: () = (*self).await;
            // `.read()` should have panicked if not in the correct scope as well
            unreachable!("Future should cancel when ready");
        }
        maybe_drop.alive(read.map(|e| std::cell::Ref::map(e, |option| option.as_ref().unwrap())))
    }
}

pub trait MaybeDrop<T>
where
    Self: Sized,
{
    type Out;

    fn alive(self, v: T) -> Self::Out;
}

impl<T> MaybeDrop<T> for () {
    type Out = T;

    fn alive(self, v: T) -> Self::Out {
        v
    }
}

impl<T, A> MaybeDrop<T> for (A,) {
    type Out = (T, A);

    fn alive(self, v: T) -> Self::Out {
        (v, self.0)
    }
}

impl<T, A, B> MaybeDrop<T> for (A, B) {
    type Out = (T, A, B);

    fn alive(self, v: T) -> Self::Out {
        (v, self.0, self.1)
    }
}

impl<T, A, B, C> MaybeDrop<T> for (A, B, C) {
    type Out = (T, A, B, C);

    fn alive(self, v: T) -> Self::Out {
        (v, self.0, self.1, self.2)
    }
}

impl<T, A, B, C, D> MaybeDrop<T> for (A, B, C, D) {
    type Out = (T, A, B, C, D);

    fn alive(self, v: T) -> Self::Out {
        (v, self.0, self.1, self.2, self.3)
    }
}

impl<T, A, B, C, D, E> MaybeDrop<T> for (A, B, C, D, E) {
    type Out = (T, A, B, C, D, E);

    fn alive(self, v: T) -> Self::Out {
        (v, self.0, self.1, self.2, self.3, self.4)
    }
}

impl<T, A, B, C, D, E, F> MaybeDrop<T> for (A, B, C, D, E, F) {
    type Out = (T, A, B, C, D, E, F);

    fn alive(self, v: T) -> Self::Out {
        (v, self.0, self.1, self.2, self.3, self.4, self.5)
    }
}

impl<T, A, B, C, D, E, F, G> MaybeDrop<T> for (A, B, C, D, E, F, G) {
    type Out = (T, A, B, C, D, E, F, G);

    fn alive(self, v: T) -> Self::Out {
        (v, self.0, self.1, self.2, self.3, self.4, self.5, self.6)
    }
}

impl<T, A, B, C, D, E, F, G, H> MaybeDrop<T> for (A, B, C, D, E, F, G, H) {
    type Out = (T, A, B, C, D, E, F, G, H);

    fn alive(self, v: T) -> Self::Out {
        (
            v, self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
        )
    }
}

impl<T, E> Resource<Result<T, E>> {
    /// Convert the `Resource<Result<T, E>>` into an `Option<Result<MappedSignal<T>, MappedSignal<E>>>`
    #[allow(clippy::type_complexity)]
    pub fn result(
        &self,
    ) -> Option<
        Result<
            MappedSignal<T, Signal<Option<Result<T, E>>>>,
            MappedSignal<E, Signal<Option<Result<T, E>>>>,
        >,
    > {
        let value: MappedSignal<T, Signal<Option<Result<T, E>>>> = self.value.map(|v| match v {
            Some(Ok(ref res)) => res,
            _ => panic!("Resource is not ready"),
        });

        let error: MappedSignal<E, Signal<Option<Result<T, E>>>> = self.value.map(|v| match v {
            Some(Err(ref err)) => err,
            _ => panic!("Resource is not ready"),
        });

        match &*self.value.peek() {
            Some(Ok(_)) => Some(Ok(value)),
            Some(Err(_)) => Some(Err(error)),
            None => None,
        }
    }
}

impl<T> From<Resource<T>> for ReadSignal<Option<T>> {
    fn from(val: Resource<T>) -> Self {
        val.value.into()
    }
}

impl<T> Readable for Resource<T> {
    type Target = Option<T>;
    type Storage = UnsyncStorage;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.value.try_read_unchecked()
    }

    #[track_caller]
    fn try_peek_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.value.try_peek_unchecked()
    }

    fn subscribers(&self) -> Subscribers {
        self.value.subscribers()
    }
}

impl<T> Writable for Resource<T> {
    type WriteMetadata = <Signal<Option<T>> as Writable>::WriteMetadata;

    fn try_write_unchecked(
        &self,
    ) -> Result<WritableRef<'static, Self>, generational_box::BorrowMutError>
    where
        Self::Target: 'static,
    {
        self.value.try_write_unchecked()
    }
}

impl<T> IntoAttributeValue for Resource<T>
where
    T: Clone + IntoAttributeValue,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T> IntoDynNode for Resource<T>
where
    T: Clone + IntoDynNode,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self().into_dyn_node()
    }
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<T: Clone> Deref for Resource<T> {
    type Target = dyn Fn() -> Option<T>;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<T> std::future::Future for Resource<T> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.waker.clone().poll_unpin(cx) {
            std::task::Poll::Ready(_) => std::task::Poll::Ready(()),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
