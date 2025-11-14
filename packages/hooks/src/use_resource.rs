use std::{
    fmt::{Debug, Display},
    future::Future,
    ops::Deref,
};

use dioxus_core::{spawn, use_hook, ReactiveContext, RenderError, SuspendedFuture, Task};
use dioxus_signals::{
    BorrowError, CopyValue, Global, InitializeFromFunction, MappedMutSignal, Readable, ReadableExt,
    ReadableRef, WritableExt, WriteSignal,
};
use dioxus_stores::{MappedStore, Store};
use futures_util::{pin_mut, FutureExt, StreamExt};

#[track_caller]
pub fn use_resource<T, F>(future: impl FnMut() -> F + 'static) -> PendingResource<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let location = std::panic::Location::caller();
    use_hook(|| Resource::new_with_location(future, location))
}

fn run_future_in_context<T, F>(
    rc: &ReactiveContext,
    mut future: impl FnMut() -> F,
    location: &'static std::panic::Location<'static>,
) -> Task
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let rc = rc.clone();
    // Create the user's task
    let fut = rc.reset_and_run_in(&mut future);

    // Spawn a wrapper task that polls the inner future and watches its dependencies
    spawn(async move {
        // Move the future here and pin it so we can poll it
        let fut = fut;
        pin_mut!(fut);

        // Run each poll in the context of the reactive scope
        // This ensures the scope is properly subscribed to the future's dependencies
        std::future::poll_fn(|cx| {
            rc.run_in(|| {
                tracing::trace_span!("polling resource", location = %location)
                    .in_scope(|| fut.poll_unpin(cx))
            })
        })
        .await;
    })
}

struct ResourceHandle {
    task: Task,
    rc: ReactiveContext,
    wakers: Vec<std::task::Waker>,
}

pub type PendingResource<T> = Resource<Store<Option<T>>>;
pub type ResolvedResource<T, Lens = WriteSignal<Option<T>>> = Resource<MappedStore<T, Lens>>;
pub type OkResource<T, E, Lens = WriteSignal<Option<T>>> =
    Resource<MappedStore<T, MappedMutSignal<Result<T, E>, Lens>>>;
pub type ErrResource<T, E, Lens = WriteSignal<Option<T>>> =
    Resource<MappedStore<E, MappedMutSignal<Result<T, E>, Lens>>>;

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
pub struct Resource<S> {
    state: S,
    handle: CopyValue<ResourceHandle>,
}

impl<S> Clone for Resource<S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Resource {
            state: self.state.clone(),
            handle: self.handle,
        }
    }
}

impl<S> Copy for Resource<S> where S: Copy {}

impl<S> Resource<S> {
    fn replace_state<S2>(self) -> impl FnOnce(S2) -> Resource<S2> {
        move |new_state| Resource {
            state: new_state,
            handle: self.handle,
        }
    }

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
    pub fn restart(&self) {
        self.handle.read().rc.mark_dirty();
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
    pub fn cancel(&self) {
        self.handle.read().task.cancel();
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
    pub fn pause(&self) {
        self.handle.read().task.pause();
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
    pub fn resume(&self) {
        self.handle.read().task.resume();
    }

    /// Get a handle to the inner task backing this resource
    /// Modify the task through this handle will cause inconsistent state
    pub fn task(&self) -> Task {
        self.handle.read().task
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
        !self.handle.read().task.paused()
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
    pub fn value(&self) -> S
    where
        S: Clone,
    {
        self.state.clone()
    }
}

impl<T: 'static> PendingResource<T> {
    #[track_caller]
    pub fn new<F>(future: impl FnMut() -> F + 'static) -> Self
    where
        F: Future<Output = T> + 'static,
    {
        let location = std::panic::Location::caller();
        Self::new_with_location(future, location)
    }

    pub fn new_with_location<F>(
        mut future: impl FnMut() -> F + 'static,
        location: &'static std::panic::Location<'static>,
    ) -> Self
    where
        F: Future<Output = T> + 'static,
    {
        let mut state = Store::new(None);
        let mut future = move || {
            let fut = future();
            async move {
                let result = fut.await;
                state.set(Some(result));
            }
        };
        let (rc, mut changed) = ReactiveContext::new();

        // Start the initial task
        let mut task = run_future_in_context(&rc, &mut future, location);
        let handle = ResourceHandle {
            task: task.clone(),
            wakers: Vec::new(),
            rc: rc.clone(),
        };
        let mut handle = CopyValue::new(handle);

        // Spawn a task to watch for changes
        spawn(async move {
            loop {
                // Wait for the dependencies to change
                let _ = changed.next().await;

                // Stop the old task
                task.cancel();

                // Start a new task
                task = run_future_in_context(
                    &rc,
                    &mut || {
                        let future = future();
                        async move {
                            let result = future.await;
                            let wakers = std::mem::take(&mut handle.write().wakers);
                            for waker in wakers {
                                waker.wake();
                            }
                            result
                        }
                    },
                    location,
                );
                let mut handle = handle.write();
                handle.task = task.clone();
            }
        });
        Resource { state, handle }
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
        self.state.set(None);
    }
}

impl<S: Readable<Target: Clone + 'static> + 'static> Deref for Resource<S> {
    type Target = dyn Fn() -> S::Target;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(&self.state) }
    }
}

impl<S: Display> Display for Resource<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.state.fmt(f)
    }
}

impl<T: Debug> Debug for Resource<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.state.fmt(f)
    }
}

impl<S: Readable> Readable for Resource<S> {
    type Target = S::Target;
    type Storage = S::Storage;

    fn try_read_unchecked(&self) -> std::result::Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.state.try_read_unchecked()
    }

    fn try_peek_unchecked(&self) -> std::result::Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.state.try_peek_unchecked()
    }

    fn subscribers(&self) -> dioxus_core::Subscribers
    where
        Self::Target: 'static,
    {
        self.state.subscribers()
    }
}

impl<T: 'static, E: 'static, Lens> Resource<Store<Option<Result<T, E>>, Lens>>
where
    Lens: Readable<Target = Option<Result<T, E>>> + Copy + 'static,
{
    /// Convert the `Resource<Result<T, E>>` into an `Option<Result<OkResource<T>, ErrResource<E>>>`
    pub fn result(&self) -> Option<Result<OkResource<T, E, Lens>, ErrResource<T, E, Lens>>> {
        self.transpose()
            .map(|store_transposed| store_transposed.transpose())
    }

    pub fn ready(&self) -> Result<OkResource<T, E, Lens>, RenderError>
    where
        E: Clone + Into<RenderError>,
        Lens: 'static,
    {
        self.suspend()?
            .transpose()
            .map_err(|err_store| err_store().into())
    }

    pub fn ok(&self) -> Result<Option<OkResource<T, E, Lens>>, RenderError>
    where
        E: Clone + Into<RenderError>,
        Lens: 'static,
    {
        match self.result() {
            None => Ok(None),
            Some(Ok(ok_store)) => Ok(Some(ok_store)),
            Some(Err(err_store)) => Err(err_store().into()),
        }
    }
}

impl<T: 'static, Lens> Resource<Store<Option<T>, Lens>>
where
    Lens: Readable<Target = Option<T>> + Copy + 'static,
{
    /// Is the resource's value ready?
    pub fn resolved(&self) -> bool {
        self.state.is_some()
    }

    /// Is the resource's value currently running?
    pub fn pending(&self) -> bool {
        self.state.is_none()
    }

    /// Suspend the resource's future and only continue rendering when the future is ready
    pub fn suspend(&self) -> Result<ResolvedResource<T, Lens>, RenderError> {
        self.transpose()
            .ok_or_else(|| RenderError::Suspended(SuspendedFuture::new(self.handle.read().task)))
    }

    pub fn transpose(&self) -> Option<ResolvedResource<T, Lens>> {
        self.state.transpose().map(self.replace_state())
    }
}

impl<T: 'static, E: 'static, Lens> Resource<Store<Result<T, E>, Lens>>
where
    Lens: Readable<Target = Result<T, E>> + Copy + 'static,
{
    pub fn transpose(
        &self,
    ) -> Result<Resource<MappedStore<T, Lens>>, Resource<MappedStore<E, Lens>>> {
        self.state
            .transpose()
            .map(self.replace_state())
            .map_err(self.replace_state())
    }
}

impl<T> std::future::IntoFuture for Resource<T> {
    type Output = ();
    type IntoFuture = ResourceFuture;

    fn into_future(self) -> Self::IntoFuture {
        ResourceFuture {
            resource: self.handle,
        }
    }
}

/// A future that is awaiting the resolution of a resource
pub struct ResourceFuture {
    resource: CopyValue<ResourceHandle>,
}

impl std::future::Future for ResourceFuture {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let myself = self.get_mut();
        let mut handle = myself.resource.write();
        if !handle.task.paused() {
            std::task::Poll::Ready(())
        } else {
            handle.wakers.push(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}

/// A type alias for global stores
///
/// # Example
/// ```rust, no_run
/// use dioxus::prelude::*;
///
/// static DOGS: GlobalResource<dioxus::Result<String>, _> = Global::new(|| async {
///     Ok(reqwest::get("https://dog.ceo/api/breeds/list/all")
///         .await?
///         .text()
///         .await?)
/// });
///
/// fn app() -> Element {
///     let dogs = DOGS.resolve();
///     match dogs.result() {
///         None => rsx! { "Loading..." },
///         Some(Ok(dogs)) => rsx! { "Dogs: {dogs}" },
///         Some(Err(err)) => rsx! { "Error: {err}" },
///     }
/// }
/// ```
pub type GlobalResource<T, F> = Global<PendingResource<T>, F>;

impl<F, T> InitializeFromFunction<F> for PendingResource<T>
where
    F: Future<Output = T> + 'static,
    T: 'static,
{
    #[track_caller]
    fn initialize_from_function(f: fn() -> F) -> Self {
        Resource::new(f)
    }
}
