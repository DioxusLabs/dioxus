use dioxus_core::{
    spawn, use_hook, Callback, IntoAttributeValue, IntoDynNode, ReactiveContext, Subscribers, Task,
};
use dioxus_core::{CapturedError, RenderError, Result, SuspendedFuture};
use dioxus_hooks::{use_callback, use_signal};
use dioxus_signals::{
    read_impls, ReadSignal, Readable, ReadableBoxExt, ReadableExt, ReadableRef, Signal, Writable,
    WritableExt, WriteLock,
};
use futures_util::{future, pin_mut, FutureExt, StreamExt};
use generational_box::{BorrowResult, UnsyncStorage};
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;
use std::{cell::Cell, ops::Deref, rc::Rc};

/// A hook to create a resource that loads data asynchronously.
///
/// This hook takes a closure that returns a future. This future will be executed on both the client
/// and the server. The loader will return `Loading` until the future resolves, at which point it will
/// return a `Loader<T>`. If the future fails, it will return `Loading::Failed`.
///
/// After the loader has successfully loaded once, it will never suspend the component again, but will
/// instead re-load the value in the background whenever any of its dependencies change.
///
/// If an error occurs while re-loading, `use_loader` will once again emit a `Loading::Failed` value.
/// The `use_loader` hook will never return a suspended state after the initial load.
///
/// # On the server
///
/// On the server, this hook will block the rendering of the component (and therefore, the page) until
/// the future resolves. Any server futures called by `use_loader` will receive the same request context
/// as the component that called `use_loader`.
#[allow(clippy::result_large_err)]
#[track_caller]
pub fn use_loader<F, T, E>(mut future: impl FnMut() -> F + 'static) -> Result<Loader<T>, Loading>
where
    F: Future<Output = Result<T, E>> + 'static,
    T: 'static + std::cmp::PartialEq + Serialize + DeserializeOwned,
    E: Into<dioxus_core::Error> + 'static,
{
    let location = std::panic::Location::caller();

    // todo: wire up serialize context into loader
    // let serialize_context = use_hook(crate::transport::serialize_context);
    // // We always create a storage entry, even if the data isn't ready yet to make it possible to deserialize pending server futures on the client
    // let storage_entry: SerializeContextEntry<T> = use_hook(|| serialize_context.create_entry());
    // use crate::{transport::SerializeContextEntry, use_server_future, use_server_future_unsuspended};

    let mut err = use_signal(|| None as Option<CapturedError>);
    let mut value = use_signal(|| None as Option<T>);
    let mut state = use_signal(|| LoaderState::Pending);
    let (rc, changed) = use_hook(|| {
        let (rc, changed) = ReactiveContext::new_with_origin(location);
        (rc, Rc::new(Cell::new(Some(changed))))
    });

    let callback = use_callback(move |_| {
        // Set the state to Pending when the task is restarted
        state.set(LoaderState::Pending);

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

            // Map the error to the captured error type so it's cheap to clone and pass out
            // We go through the anyhow::Error first since it's more useful.
            let res: Result<T, CapturedError> = res.map_err(|e| {
                let res: dioxus_core::Error = e.into();
                res.into()
            });

            match res {
                Ok(v) => {
                    err.set(None);
                    value.set(Some(v));
                    state.set(LoaderState::Ready);
                }
                Err(e) => {
                    err.set(Some(e));
                    state.set(LoaderState::Failed);
                }
            }
        })
    });

    let mut task = use_hook(|| Signal::new(callback(())));

    use_hook(|| {
        let mut changed = changed.take().unwrap();
        spawn(async move {
            loop {
                // Wait for the dependencies to change
                let _ = changed.next().await;

                // Stop the old task
                task.write().cancel();

                // Start a new task
                task.set(callback(()));
            }
        })
    });

    let read_value = use_hook(|| value.map(|f| f.as_ref().unwrap()).boxed());

    let handle = LoaderHandle {
        task,
        err,
        callback,
        state,
    };

    match &*state.read_unchecked() {
        LoaderState::Pending => Err(Loading::Pending(handle)),
        LoaderState::Failed => Err(Loading::Failed(handle)),
        LoaderState::Ready => Ok(Loader {
            real_value: value,
            read_value,
            error: err,
            state,
            task,
            handle,
        }),
    }
}

/// A Loader is a signal that represents a value that is loaded asynchronously.
///
/// Once a `Loader<T>` has been successfully created from `use_loader`, it can be use like a normal signal of type `T`.
///
/// When the loader is re-reloading its values, it will no longer suspend its component, making it
/// very useful for server-side-rendering.
pub struct Loader<T: 'static> {
    /// This is a signal that unwraps the inner value. We can't give it out unless we know the inner value is Some(T)!
    read_value: ReadSignal<T>,

    /// This is the actual signal. We let the user set this value if they want to, but we can't let them set it to `None`.
    real_value: Signal<Option<T>>,
    error: Signal<Option<CapturedError>>,
    state: Signal<LoaderState>,
    task: Signal<Task>,
    handle: LoaderHandle,
}

impl<T: 'static> Loader<T> {
    /// Get the error that occurred during loading, if any.
    ///
    /// After initial load, this will return `None` until the next reload fails.
    pub fn error(&self) -> Option<CapturedError> {
        self.error.read().as_ref().cloned()
    }

    /// Restart the loading task.
    ///
    /// After initial load, this won't suspend the component, but will reload in the background.
    pub fn restart(&mut self) {
        self.handle.restart();
    }

    /// Check if the loader has failed.
    pub fn is_error(&self) -> bool {
        self.error.read().is_some() && matches!(*self.state.read(), LoaderState::Failed)
    }

    /// Cancel the current loading task.
    pub fn cancel(&self) {
        self.task.read().cancel();
    }
}

impl<T: 'static> Writable for Loader<T> {
    type WriteMetadata = <Signal<Option<T>> as Writable>::WriteMetadata;

    fn try_write_unchecked(
        &self,
    ) -> std::result::Result<
        dioxus_signals::WritableRef<'static, Self>,
        generational_box::BorrowMutError,
    >
    where
        Self::Target: 'static,
    {
        let writer = self.real_value.try_write_unchecked()?;
        Ok(WriteLock::map(writer, |f: &mut Option<T>| {
            f.as_mut()
                .expect("Loader value should be set if the `Loader<T>` exists")
        }))
    }
}

impl<T> Readable for Loader<T> {
    type Target = T;
    type Storage = UnsyncStorage;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>
    where
        T: 'static,
    {
        Ok(self.read_value.read_unchecked())
    }

    /// Get the current value of the signal. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>>
    where
        T: 'static,
    {
        Ok(self.peek_unchecked())
    }

    fn subscribers(&self) -> Subscribers
    where
        T: 'static,
    {
        self.read_value.subscribers()
    }
}

impl<T> IntoAttributeValue for Loader<T>
where
    T: Clone + IntoAttributeValue + PartialEq + 'static,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T> IntoDynNode for Loader<T>
where
    T: Clone + IntoDynNode + PartialEq + 'static,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self().into_dyn_node()
    }
}

impl<T: 'static> PartialEq for Loader<T> {
    fn eq(&self, other: &Self) -> bool {
        self.read_value == other.read_value
    }
}

impl<T: Clone> Deref for Loader<T>
where
    T: PartialEq + 'static,
{
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

read_impls!(Loader<T> where T: PartialEq);

impl<T> Clone for Loader<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Loader<T> {}

#[derive(Clone, Copy, PartialEq, Hash, Eq, Debug)]
pub enum LoaderState {
    /// The loader's future is still running
    Pending,

    /// The loader's future has completed successfully
    Ready,

    /// The loader's future has failed and now the loader is in an error state.
    Failed,
}

#[derive(PartialEq)]
pub struct LoaderHandle {
    callback: Callback<(), Task>,
    task: Signal<Task>,
    err: Signal<Option<CapturedError>>,
    state: Signal<LoaderState>,
}

impl LoaderHandle {
    /// Restart the loading task.
    pub fn restart(&mut self) {
        self.task.write().cancel();
        let new_task = self.callback.call(());
        self.task.set(new_task);
    }

    /// Get the current state of the loader.
    pub fn state(&self) -> LoaderState {
        *self.state.read()
    }
}

impl Clone for LoaderHandle {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for LoaderHandle {}

#[derive(PartialEq)]
pub enum Loading {
    /// The loader is still pending and the component should suspend.
    Pending(LoaderHandle),

    /// The loader has failed and an error will be returned up the tree.
    Failed(LoaderHandle),
}

impl std::fmt::Debug for Loading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loading::Pending(_) => write!(f, "Loading::Pending"),
            Loading::Failed(_) => write!(f, "Loading::Failed"),
        }
    }
}

impl std::fmt::Display for Loading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loading::Pending(_) => write!(f, "Loading is still pending"),
            Loading::Failed(_) => write!(f, "Loading has failed"),
        }
    }
}

/// Convert a Loading into a RenderError for use with the `?` operator in components
impl From<Loading> for RenderError {
    fn from(val: Loading) -> Self {
        match val {
            Loading::Pending(t) => RenderError::Suspended(SuspendedFuture::new(t.task.cloned())),
            Loading::Failed(err) => RenderError::Error(
                err.err
                    .cloned()
                    .expect("LoaderHandle in Failed state should always have an error"),
            ),
        }
    }
}
