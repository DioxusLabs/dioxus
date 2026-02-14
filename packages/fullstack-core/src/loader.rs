use dioxus_core::{use_hook, IntoAttributeValue, IntoDynNode, Subscribers};
use dioxus_core::{CapturedError, RenderError, Result, SuspendedFuture};
use dioxus_hooks::{use_resource, use_signal, Resource};
use dioxus_signals::{
    read_impls, ReadSignal, Readable, ReadableBoxExt, ReadableExt, ReadableRef, Signal, Writable,
    WritableExt, WriteLock,
};
use generational_box::{BorrowResult, UnsyncStorage};
use serde::{de::DeserializeOwned, Serialize};
use std::ops::Deref;
use std::{cmp::PartialEq, future::Future};

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
    T: 'static + PartialEq + Serialize + DeserializeOwned,
    E: Into<CapturedError> + 'static,
{
    let serialize_context = use_hook(crate::transport::serialize_context);

    // We always create a storage entry, even if the data isn't ready yet to make it possible to deserialize pending server futures on the client
    #[allow(unused)]
    let storage_entry: crate::transport::SerializeContextEntry<Result<T, CapturedError>> =
        use_hook(|| serialize_context.create_entry());

    #[cfg(feature = "server")]
    let caller = std::panic::Location::caller();

    // If this is the first run and we are on the web client, the data might be cached
    #[cfg(feature = "web")]
    let initial_web_result =
        use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(Some(storage_entry.get()))));

    let mut error = use_signal(|| None as Option<CapturedError>);
    let mut value = use_signal(|| None as Option<T>);
    let mut loader_state = use_signal(|| LoaderState::Pending);

    let resource = use_resource(move || {
        #[cfg(feature = "server")]
        let storage_entry = storage_entry.clone();

        let user_fut = future();

        #[cfg(feature = "web")]
        let initial_web_result = initial_web_result.clone();

        #[allow(clippy::let_and_return)]
        async move {
            // If this is the first run and we are on the web client, the data might be cached
            #[cfg(feature = "web")]
            match initial_web_result.take() {
                // The data was deserialized successfully from the server
                Some(Ok(o)) => {
                    match o {
                        Ok(v) => {
                            value.set(Some(v));
                            loader_state.set(LoaderState::Ready);
                        }
                        Err(e) => {
                            error.set(Some(e));
                            loader_state.set(LoaderState::Failed);
                        }
                    };
                    return;
                }

                // The data is still pending from the server. Don't try to resolve it on the client
                Some(Err(crate::transport::TakeDataError::DataPending)) => {
                    std::future::pending::<()>().await
                }

                // The data was not available on the server, rerun the future
                Some(Err(_)) => {}

                // This isn't the first run, so we don't need do anything
                None => {}
            }

            // Otherwise just run the future itself
            let out = user_fut.await;

            // Remap the error to the captured error type so it's cheap to clone and pass out, just
            // slightly more cumbersome to access the inner error.
            let out = out.map_err(|e| {
                let anyhow_err: CapturedError = e.into();
                anyhow_err
            });

            // If this is the first run and we are on the server, cache the data in the slot we reserved for it
            #[cfg(feature = "server")]
            storage_entry.insert(&out, caller);

            match out {
                Ok(v) => {
                    value.set(Some(v));
                    loader_state.set(LoaderState::Ready);
                }
                Err(e) => {
                    error.set(Some(e));
                    loader_state.set(LoaderState::Failed);
                }
            };
        }
    });

    // On the first run, force this task to be polled right away in case its value is ready
    use_hook(|| {
        let _ = resource.task().poll_now();
    });

    let read_value = use_hook(|| value.map(|f| f.as_ref().unwrap()).boxed());

    let handle = LoaderHandle {
        resource,
        error,
        state: loader_state,
        _marker: std::marker::PhantomData,
    };

    match &*loader_state.read_unchecked() {
        LoaderState::Pending => Err(Loading::Pending(handle)),
        LoaderState::Failed => Err(Loading::Failed(handle)),
        LoaderState::Ready => Ok(Loader {
            real_value: value,
            read_value,
            error,
            state: loader_state,
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
    pub fn cancel(&mut self) {
        self.handle.resource.cancel();
    }

    pub fn loading(&self) -> bool {
        !self.handle.resource.finished()
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
        Ok(self.read_value.peek_unchecked())
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
        let t: T = self();
        t.into_dyn_node()
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
        dioxus_signals::readable_deref_impl(self)
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
pub struct LoaderHandle<M = ()> {
    resource: Resource<()>,
    error: Signal<Option<CapturedError>>,
    state: Signal<LoaderState>,
    _marker: std::marker::PhantomData<M>,
}

impl LoaderHandle {
    /// Restart the loading task.
    pub fn restart(&mut self) {
        self.resource.restart();
    }

    /// Get the current state of the loader.
    pub fn state(&self) -> LoaderState {
        *self.state.read()
    }

    pub fn error(&self) -> Option<CapturedError> {
        self.error.read().as_ref().cloned()
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
            Loading::Pending(t) => RenderError::Suspended(SuspendedFuture::new(t.resource.task())),
            Loading::Failed(err) => RenderError::Error(
                err.error
                    .cloned()
                    .expect("LoaderHandle in Failed state should always have an error"),
            ),
        }
    }
}
