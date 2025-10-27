use std::{fmt::Display, future::Future, ops::Deref};

use dioxus_core::{spawn, use_hook, ReactiveContext, RenderError, SuspendedFuture, Task};
use dioxus_signals::{
    BorrowError, CopyValue, MappedMutSignal, Readable, ReadableExt, ReadableRef, WritableExt,
    WriteSignal,
};
use dioxus_stores::{MappedStore, Store};
use futures_util::{pin_mut, FutureExt, StreamExt};

#[track_caller]
pub fn use_resource<T, F>(future: impl FnMut() -> F + 'static) -> ClasicResource<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let location = std::panic::Location::caller();
    use_hook(|| create_resource(future, location))
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

#[derive(Store)]
struct ResourceHandle {
    task: Task,
    rc: ReactiveContext,
    wakers: Vec<std::task::Waker>,
}

pub type ClasicResource<T> = Resource<Store<Option<T>>>;
pub type ResolvedResource<T, Lens = WriteSignal<Option<T>>> = Resource<MappedStore<T, Lens>>;
pub type OkResource<T, E, Lens = WriteSignal<Option<T>>> =
    Resource<MappedStore<T, MappedMutSignal<Result<T, E>, Lens>>>;
pub type ErrResource<T, E, Lens = WriteSignal<Option<T>>> =
    Resource<MappedStore<E, MappedMutSignal<Result<T, E>, Lens>>>;

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

    pub fn restart(&self) {
        self.handle.read().rc.mark_dirty();
    }

    pub fn running(&self) -> bool {
        !self.handle.read().task.paused()
    }

    pub fn cancel(&self) {
        self.handle.read().task.cancel();
    }

    pub fn pause(&self) {
        self.handle.read().task.pause();
    }

    pub fn resume(&self) {
        self.handle.read().task.resume();
    }

    pub fn task(&self) -> Task {
        self.handle.read().task
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

fn create_resource<T, F>(
    mut future: impl FnMut() -> F + 'static,
    location: &'static std::panic::Location<'static>,
) -> Resource<Store<Option<T>>>
where
    T: 'static,
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

impl<T: 'static, E: 'static, Lens> Resource<Store<Option<Result<T, E>>, Lens>>
where
    Lens: Readable<Target = Option<Result<T, E>>> + Copy + 'static,
{
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
    pub fn resolved(&self) -> bool {
        self.state.is_some()
    }

    pub fn pending(&self) -> bool {
        self.state.is_none()
    }

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
