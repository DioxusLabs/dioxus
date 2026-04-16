use std::future::Future;

use dioxus_core::{
    spawn, use_hook, ReactiveContext, RenderError, Subscribers, SuspendedFuture, Task,
};
use dioxus_optics::{
    AsFuture, Combinator, ErrPrism, FutureAccess, OkPrism, Optic, PrismOp, Required, SomePrism,
};
use dioxus_signals::{
    BorrowError, BorrowMutError, CopyValue, Global, InitializeFromFunction, MappedMutSignal,
    Readable, ReadableExt, ReadableRef, Writable, WritableExt, WritableRef, WriteSignal,
};
use dioxus_stores::Store;
use futures_util::{pin_mut, FutureExt, StreamExt};

// ---------------------------------------------------------------------------
// `use_resource` hook
// ---------------------------------------------------------------------------

/// Spawn a reactive future and return a handle to its result.
#[track_caller]
pub fn use_resource<T, F>(future: impl FnMut() -> F + 'static) -> PendingResource<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let location = std::panic::Location::caller();
    use_hook(|| new_pending_resource(future, location))
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
    let fut = rc.reset_and_run_in(&mut future);

    spawn(async move {
        let fut = fut;
        pin_mut!(fut);
        std::future::poll_fn(|cx| {
            rc.run_in(|| {
                tracing::trace_span!("polling resource", location = %location)
                    .in_scope(|| fut.poll_unpin(cx))
            })
        })
        .await;
    })
}

/// Internal handle to the task + reactive context backing a resource.
pub struct ResourceHandle {
    task: Task,
    rc: ReactiveContext,
    wakers: Vec<std::task::Waker>,
}

// ---------------------------------------------------------------------------
// HandledLens — a transparent lens wrapper that carries a resource handle.
//
// `Resource<T, L>` keeps the normal `Store` carrier, but swaps in a
// `HandledLens<L>` at the bottom of the lens chain. Store projections compose
// via mapped child lenses that preserve the inner lens verbatim, so the handle
// rides through every projection automatically.
// ---------------------------------------------------------------------------

/// Lens wrapper that attaches a [`ResourceHandle`] to an underlying signal.
/// Readable/Writable delegate to `inner`; the handle is reached via
/// [`ResourceLike`].
pub struct HandledLens<L> {
    inner: L,
    handle: CopyValue<ResourceHandle>,
}

impl<L: Copy> Copy for HandledLens<L> {}
impl<L: Clone> Clone for HandledLens<L> {
    fn clone(&self) -> Self {
        HandledLens {
            inner: self.inner.clone(),
            handle: self.handle,
        }
    }
}
impl<L: PartialEq> PartialEq for HandledLens<L> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<L: Readable> Readable for HandledLens<L> {
    type Target = L::Target;
    type Storage = L::Storage;
    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.inner.try_read_unchecked()
    }
    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.inner.try_peek_unchecked()
    }
    fn subscribers(&self) -> Subscribers
    where
        Self::Target: 'static,
    {
        self.inner.subscribers()
    }
}

impl<L: Writable> Writable for HandledLens<L> {
    type WriteMetadata = L::WriteMetadata;
    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError> {
        self.inner.try_write_unchecked()
    }
}

// ---------------------------------------------------------------------------
// ResourceLike — "this lens chain bottoms out at a HandledLens".
// ---------------------------------------------------------------------------

/// Trait satisfied by any lens whose chain terminates at a [`HandledLens`].
pub trait ResourceLike: Readable + Copy {
    fn resource_handle(&self) -> CopyValue<ResourceHandle>;
}

impl<L: Readable + Copy> ResourceLike for HandledLens<L> {
    fn resource_handle(&self) -> CopyValue<ResourceHandle> {
        self.handle
    }
}

impl<O: ?Sized, V, F, FMut> ResourceLike for MappedMutSignal<O, V, F, FMut>
where
    V: ResourceLike,
    Self: Readable + Copy,
{
    fn resource_handle(&self) -> CopyValue<ResourceHandle> {
        self.inner().resource_handle()
    }
}

// ---------------------------------------------------------------------------
// Public type aliases — resources reuse the normal store carrier. The optic
// helpers (`map_some`, `map_ok`, `map_err`, `each`, `each_hash_map`, …) lift
// onto resources automatically through the `Access` / `Pathed` traits in
// `dioxus_optics`.
// ---------------------------------------------------------------------------

/// A reactive async value. `Resource` is a `Store` whose lens carries a
/// resource handle.
pub type Resource<T, L = WriteSignal<T>> = Store<T, HandledLens<L>>;

/// A resource whose future hasn't necessarily resolved yet.
pub type PendingResource<T> = Resource<Option<T>>;

/// A resource projected to its resolved inner value via the optics
/// `Some` prism.
pub type ResolvedResource<T, Lens = WriteSignal<Option<T>>> = Optic<
    Combinator<Store<Option<T>, HandledLens<Lens>>, PrismOp<SomePrism<T>>>,
    Required,
>;

/// Projection of a resolved resource into its `Ok` branch via the optics
/// `Some` + `Ok` prisms.
pub type OkResource<T, E, Lens = WriteSignal<Option<Result<T, E>>>> = Optic<
    Combinator<
        Combinator<
            Store<Option<Result<T, E>>, HandledLens<Lens>>,
            PrismOp<SomePrism<Result<T, E>>>,
        >,
        PrismOp<OkPrism<T, E>>,
    >,
    Required,
>;

/// Projection of a resolved resource into its `Err` branch via the
/// optics `Some` + `Err` prisms.
pub type ErrResource<T, E, Lens = WriteSignal<Option<Result<T, E>>>> = Optic<
    Combinator<
        Combinator<
            Store<Option<Result<T, E>>, HandledLens<Lens>>,
            PrismOp<SomePrism<Result<T, E>>>,
        >,
        PrismOp<ErrPrism<T, E>>,
    >,
    Required,
>;

// ---------------------------------------------------------------------------
// HasResourceHandle — finds the resource handle through a Store carrier or
// through any optic chain (Combinator / Optic) wrapping one.
// ---------------------------------------------------------------------------

/// Anything that can surface a [`ResourceHandle`] — either a `Store<_, L>`
/// where `L: ResourceLike`, or any optics chain (`Combinator` / `Optic`)
/// whose root is such a store.
pub trait HasResourceHandle {
    fn resource_handle(&self) -> CopyValue<ResourceHandle>;
}

impl<T: ?Sized, L: ResourceLike> HasResourceHandle for Store<T, L>
where
    L::Target: 'static,
{
    fn resource_handle(&self) -> CopyValue<ResourceHandle> {
        self.lens().resource_handle()
    }
}

impl<A: HasResourceHandle, Op> HasResourceHandle for Combinator<A, Op> {
    fn resource_handle(&self) -> CopyValue<ResourceHandle> {
        self.parent().resource_handle()
    }
}

impl<A: HasResourceHandle, P> HasResourceHandle for Optic<A, P> {
    fn resource_handle(&self) -> CopyValue<ResourceHandle> {
        self.access().resource_handle()
    }
}

// ---------------------------------------------------------------------------
// ResourceControls — handle-based methods. Local trait → blanket impl OK.
// ---------------------------------------------------------------------------

/// Control methods that work on any projection of a resource.
pub trait ResourceControls {
    fn restart(&self);
    fn cancel(&self);
    fn pause(&self);
    fn resume(&self);
    fn task(&self) -> Task;
    fn finished(&self) -> bool;
    fn pending(&self) -> bool;
    /// Await the next completion of this resource's task.
    fn wait(&self) -> ResourceFuture;
}

impl<T: HasResourceHandle> ResourceControls for T {
    fn restart(&self) {
        self.resource_handle().read().rc.mark_dirty();
    }
    fn cancel(&self) {
        self.resource_handle().read().task.cancel();
    }
    fn pause(&self) {
        self.resource_handle().read().task.pause();
    }
    fn resume(&self) {
        self.resource_handle().read().task.resume();
    }
    fn task(&self) -> Task {
        self.resource_handle().read().task
    }
    fn finished(&self) -> bool {
        !self.resource_handle().read().task.paused()
    }
    fn pending(&self) -> bool {
        self.resource_handle().read().task.paused()
    }
    fn wait(&self) -> ResourceFuture {
        ResourceFuture {
            resource: self.resource_handle(),
        }
    }
}

// ---------------------------------------------------------------------------
// PendingResourceExt / FallibleResourceExt — resource-shape-specific ext methods.
//
// `transpose` / `is_some` / `is_none` come from
// `dioxus_stores::StoreOptionExt` / `StoreResultExt` (via the imports
// above). Those traits are the Store-flavored counterparts to optics'
// `Optic::try_some` / `Optic::try_ok`.
// ---------------------------------------------------------------------------

/// Methods for resources of shape `Option<T>`.
pub trait PendingResourceExt: Sized {
    type Inner: 'static;
    type InnerLens: Readable<Target = Option<Self::Inner>> + Copy + 'static;

    fn suspend(&self) -> Result<ResolvedResource<Self::Inner, Self::InnerLens>, RenderError>;
    /// Is the resource's future currently running (value still unresolved)?
    fn running(&self) -> bool;
    /// Has the resource's future resolved?
    fn resolved(&self) -> bool;
}

impl<T, Lens> PendingResourceExt for Store<Option<T>, HandledLens<Lens>>
where
    T: 'static,
    Lens: Readable<Target = Option<T>> + Copy + 'static,
{
    type Inner = T;
    type InnerLens = Lens;

    fn suspend(&self) -> Result<ResolvedResource<T, Lens>, RenderError> {
        let handle = self.lens().resource_handle();
        Optic::from_access(*self)
            .try_some::<T>()
            .ok_or_else(|| RenderError::Suspended(SuspendedFuture::new(handle.read().task)))
    }

    fn running(&self) -> bool {
        self.lens().peek_unchecked().is_none()
    }

    fn resolved(&self) -> bool {
        self.lens().peek_unchecked().is_some()
    }
}

/// Methods for resources whose value is `Option<Result<T, E>>`.
pub trait FallibleResourceExt: Sized {
    type Ok: 'static;
    type Err: 'static;
    type InnerLens: Readable<Target = Option<Result<Self::Ok, Self::Err>>> + Copy + 'static;

    fn result(
        &self,
    ) -> Option<
        Result<
            OkResource<Self::Ok, Self::Err, Self::InnerLens>,
            ErrResource<Self::Ok, Self::Err, Self::InnerLens>,
        >,
    >;

    fn ready(&self) -> Result<OkResource<Self::Ok, Self::Err, Self::InnerLens>, RenderError>
    where
        Self::Err: Clone + Into<RenderError>;

    fn ok(&self) -> Result<Option<OkResource<Self::Ok, Self::Err, Self::InnerLens>>, RenderError>
    where
        Self::Err: Clone + Into<RenderError>;
}

impl<T, E, Lens> FallibleResourceExt for Store<Option<Result<T, E>>, HandledLens<Lens>>
where
    T: 'static,
    E: 'static,
    Lens: Readable<Target = Option<Result<T, E>>> + Copy + 'static,
{
    type Ok = T;
    type Err = E;
    type InnerLens = Lens;

    fn result(&self) -> Option<Result<OkResource<T, E, Lens>, ErrResource<T, E, Lens>>> {
        Optic::from_access(*self)
            .try_some::<Result<T, E>>()
            .map(|resolved| resolved.try_ok::<T, E>())
    }

    fn ready(&self) -> Result<OkResource<T, E, Lens>, RenderError>
    where
        E: Clone + Into<RenderError>,
    {
        self.suspend()?
            .try_ok::<T, E>()
            .map_err(|err_optic| (*err_optic.read()).clone().into())
    }

    fn ok(&self) -> Result<Option<OkResource<T, E, Lens>>, RenderError>
    where
        E: Clone + Into<RenderError>,
    {
        match self.result() {
            None => Ok(None),
            Some(Ok(ok_optic)) => Ok(Some(ok_optic)),
            Some(Err(err_optic)) => Err((*err_optic.read()).clone().into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Resource future (future-of-resource-resolution).
// ---------------------------------------------------------------------------

/// A future that resolves when a resource's task next completes.
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

/// A future that resolves to the value of a resource once its task completes.
///
/// Created by [`FutureAccess::future`] on a [`HandledLens`] (the lens any
/// `Resource` carries) and exposed through any optic chain that reaches a
/// resource carrier.
pub struct ResourceValueFuture<L: Readable> {
    handle: CopyValue<ResourceHandle>,
    lens: L,
}

impl<L> std::future::Future for ResourceValueFuture<L>
where
    L: Readable + Copy + Unpin + 'static,
    L::Target: Clone + 'static,
{
    type Output = L::Target;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let myself = self.get_mut();
        let mut handle = myself.handle.write();
        if !handle.task.paused() {
            drop(handle);
            std::task::Poll::Ready(myself.lens.peek().clone())
        } else {
            handle.wakers.push(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}

impl<L> FutureAccess<AsFuture<ResourceValueFuture<HandledLens<L>>>>
    for HandledLens<L>
where
    L: Readable + Copy + Unpin + 'static,
    L::Target: Clone + Sized + 'static,
{
    fn future(&self) -> AsFuture<ResourceValueFuture<HandledLens<L>>> {
        AsFuture(ResourceValueFuture {
            handle: self.handle,
            lens: *self,
        })
    }
}

// ---------------------------------------------------------------------------
// Constructors.
// ---------------------------------------------------------------------------

/// Build a [`PendingResource`] from a future-producing closure.
#[track_caller]
pub fn new_pending_resource<T, F>(
    mut future: impl FnMut() -> F + 'static,
    location: &'static std::panic::Location<'static>,
) -> PendingResource<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let mut state: Store<Option<T>, WriteSignal<Option<T>>> = Store::new(None);
    let mut run_user_future = move || {
        let fut = future();
        async move {
            let result = fut.await;
            state.set(Some(result));
        }
    };
    let (rc, mut changed) = ReactiveContext::new();

    let mut task = run_future_in_context(&rc, &mut run_user_future, location);
    let handle = ResourceHandle {
        task: task.clone(),
        wakers: Vec::new(),
        rc: rc.clone(),
    };
    let mut handle = CopyValue::new(handle);

    spawn(async move {
        loop {
            let _ = changed.next().await;
            task.cancel();
            task = run_future_in_context(
                &rc,
                &mut || {
                    let future = run_user_future();
                    async move {
                        future.await;
                        let wakers = std::mem::take(&mut handle.write().wakers);
                        for waker in wakers {
                            waker.wake();
                        }
                    }
                },
                location,
            );
            let mut h = handle.write();
            h.task = task.clone();
        }
    });

    // Wrap the store's lens with HandledLens so resource-specific methods can
    // reach the task handle via the `ResourceLike` trait.
    let selector = state.into_selector();
    let handled = selector.map_writer(|inner| HandledLens { inner, handle });
    handled.into()
}

// ---------------------------------------------------------------------------
// Global resources — newtype wrapper around PendingResource to sidestep the
// orphan rule on `InitializeFromFunction` (its trait generic `F` isn't wrapped
// in any local type in the impl signature otherwise).
// ---------------------------------------------------------------------------

/// A globally-registered resource.
pub struct LazyResource<T: 'static>(PendingResource<T>);

impl<T: 'static> Copy for LazyResource<T> {}
impl<T: 'static> Clone for LazyResource<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> LazyResource<T> {
    /// Unwrap into the underlying [`PendingResource`].
    pub fn get(self) -> PendingResource<T> {
        self.0
    }
}

impl<T: 'static> std::ops::Deref for LazyResource<T> {
    type Target = PendingResource<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<F, T> InitializeFromFunction<F> for LazyResource<T>
where
    F: Future<Output = T> + 'static,
    T: 'static,
{
    #[track_caller]
    fn initialize_from_function(f: fn() -> F) -> Self {
        let location = std::panic::Location::caller();
        LazyResource(new_pending_resource(f, location))
    }
}

/// A type alias for global resources.
pub type GlobalResource<T, F> = Global<LazyResource<T>, F>;
