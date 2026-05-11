use std::{any::Any, ops::Deref};

use dioxus_core::{
    IntoAttributeValue, IntoDynNode, ReactiveContext, Subscribers, current_scope_id,
};
use generational_box::{BorrowResult, Storage, SyncStorage, UnsyncStorage};

use crate::{
    CopyValue, Global, InitializeFromFunction, MappedMutSignal, MappedSignal, Memo, Readable,
    ReadableExt, ReadableRef, Signal, SignalData, Writable, WritableExt, read_impls, write_impls,
};

/// A signal that can only be read from.
#[deprecated(
    since = "0.7.0",
    note = "Use `ReadSignal` instead. Will be removed in 0.8"
)]
pub type ReadOnlySignal<T, S = UnsyncStorage> = ReadSignal<T, S>;

/// The data behind a [ReadSignal] handle. The wrapped readable and the wrapper-level subscriber
/// state live in a single [CopyValue] so that creating a wrapper that is immediately `point_to`'d
/// (e.g. from the `props` macro) only allocates one generational-box reference — matching what a
/// bare `Signal` costs. `point_to` moves the wrapped readable out of `other.inner` into our own
/// `inner.value`, so subscribers attached to this wrapper's identity (the *outer* handle) stay put
/// while the underlying readable is replaced.
#[doc(hidden)]
pub struct ReadSignalInner<T: ?Sized, S: BoxedSignalStorage<T>> {
    /// The wrapped readable. `Option` so `point_to` can move ownership without needing a dummy.
    pub(crate) value: Option<Box<S::DynReadable<sealed::SealedToken>>>,
    /// Subscribers attached to this wrapper handle, independent of the wrapped readable. Lazy so
    /// wrappers that are never read pay nothing.
    pub(crate) subscribers: Option<Subscribers>,
    /// Bridges writes on the inner readable to our own subscribers so they follow the wrapper's
    /// identity across `point_to` swaps. Lazy for the same reason as `subscribers`.
    pub(crate) forwarding_context: Option<ReactiveContext>,
}

/// A boxed version of [Readable] that can be used to store any readable type.
pub struct ReadSignal<T: ?Sized, S: BoxedSignalStorage<T> = UnsyncStorage> {
    inner: CopyValue<ReadSignalInner<T, S>, S>,
}

impl<T: ?Sized + 'static> ReadSignal<T> {
    /// Create a new boxed readable value.
    pub fn new(value: impl Readable<Target = T, Storage = UnsyncStorage> + 'static) -> Self {
        Self::new_maybe_sync(value)
    }
}

impl<T: ?Sized + 'static, S: BoxedSignalStorage<T>> ReadSignal<T, S> {
    /// Create a new boxed readable value which may be sync
    pub fn new_maybe_sync<R>(value: R) -> Self
    where
        S: CreateBoxedSignalStorage<R>,
        R: Readable<Target = T>,
    {
        Self {
            inner: CopyValue::new_maybe_sync(ReadSignalInner {
                value: Some(S::new_readable(value, sealed::SealedToken)),
                subscribers: None,
                forwarding_context: None,
            }),
        }
    }

    /// Get this wrapper's subscriber list, initializing it on first use.
    fn ensure_subscribers(&self) -> Subscribers {
        if let Some(subscribers) = self.inner.try_peek_unchecked().unwrap().subscribers.clone() {
            return subscribers;
        }
        let subscribers = Subscribers::new();
        self.inner.try_write_unchecked().unwrap().subscribers = Some(subscribers.clone());
        subscribers
    }

    /// Get the reactive context that forwards writes on the inner readable to this wrapper's
    /// subscribers, initializing it on first use.
    fn ensure_forwarding_context(&self) -> ReactiveContext {
        if let Some(context) = self.inner.try_peek_unchecked().unwrap().forwarding_context {
            return context;
        }
        // Capture the `Subscribers` handle (which is `Send + Sync`) instead of the inner
        // `CopyValue`, since the forwarding callback must be `Send + Sync` regardless of `S`.
        let subscribers = self.ensure_subscribers();
        let context = ReactiveContext::new_with_callback(
            move || {
                let mut current_subscribers = Vec::new();
                subscribers.visit(|subscriber| current_subscribers.push(*subscriber));
                for subscriber in current_subscribers {
                    if !subscriber.mark_dirty() {
                        subscribers.remove(&subscriber);
                    }
                }
            },
            current_scope_id(),
            std::panic::Location::caller(),
        );
        self.inner.try_write_unchecked().unwrap().forwarding_context = Some(context);
        context
    }

    /// Point to another [ReadSignal]. Subscribers attached to this wrapper migrate to the new
    /// inner readable so they receive writes through it, while subscribers attached directly to
    /// the underlying readable are left alone.
    pub fn point_to(&self, other: Self) -> BorrowResult {
        // Migrate wrapper subscribers from the old inner readable to the new one before swapping
        // the boxed value, so the underlying signal's `subscribers` accounting stays consistent.
        let this_subscribers_clone = self.inner.try_peek_unchecked().unwrap().subscribers.clone();
        if let Some(this_subscribers) = &this_subscribers_clone {
            let old_wrapped_subscribers = self
                .inner
                .try_peek_unchecked()
                .unwrap()
                .value
                .as_ref()
                .expect("ReadSignal is missing its wrapped value")
                .subscribers();
            let other_wrapped_subscribers = other
                .inner
                .try_peek_unchecked()
                .unwrap()
                .value
                .as_ref()
                .expect("ReadSignal is missing its wrapped value")
                .subscribers();
            let mut this_subscribers_vec = Vec::new();
            // Note we don't subscribe directly in the visit closure to avoid a deadlock when pointing to self
            this_subscribers.visit(|subscriber| this_subscribers_vec.push(*subscriber));
            for subscriber in this_subscribers_vec {
                old_wrapped_subscribers.remove(&subscriber);
                subscriber.subscribe(other_wrapped_subscribers.clone());
            }
            if let Some(forwarding_context) =
                self.inner.try_peek_unchecked().unwrap().forwarding_context
            {
                forwarding_context.clear_subscribers();
                forwarding_context.subscribe(other_wrapped_subscribers);
            }
        }

        // Move the new boxed value into our slot, dropping our previous one. We keep our existing
        // `subscribers` and `forwarding_context` so the wrapper's identity is preserved.
        let new_value = other
            .inner
            .try_write_unchecked()
            .unwrap()
            .value
            .take()
            .expect("ReadSignal is missing its wrapped value");
        self.inner.try_write_unchecked().unwrap().value = Some(new_value);
        // Release `other`'s slot: `other` is consumed here and its state was empty (None) for the
        // common props-macro path, so this just frees the generational-box entry.
        other.inner.manually_drop();
        Ok(())
    }

    #[doc(hidden)]
    /// This is only used by the `props` macro.
    /// Mark any readers of the signal as dirty
    pub fn mark_dirty(&mut self) {
        let Some(subscribers) = self.inner.try_peek_unchecked().unwrap().subscribers.clone() else {
            return;
        };
        let mut this_subscribers_vec = Vec::new();
        subscribers.visit(|subscriber| this_subscribers_vec.push(*subscriber));
        for subscriber in this_subscribers_vec {
            subscribers.remove(&subscriber);
            subscriber.mark_dirty();
        }
    }
}

impl<T: ?Sized, S: BoxedSignalStorage<T>> Clone for ReadSignal<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized, S: BoxedSignalStorage<T>> Copy for ReadSignal<T, S> {}

impl<T: ?Sized, S: BoxedSignalStorage<T>> PartialEq for ReadSignal<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<
    T: Default + 'static,
    S: CreateBoxedSignalStorage<Signal<T, S>> + BoxedSignalStorage<T> + Storage<SignalData<T>>,
> Default for ReadSignal<T, S>
{
    fn default() -> Self {
        Self::new_maybe_sync(Signal::new_maybe_sync(T::default()))
    }
}

read_impls!(ReadSignal<T, S: BoxedSignalStorage<T>>);

impl<T, S: BoxedSignalStorage<T>> IntoAttributeValue for ReadSignal<T, S>
where
    T: Clone + IntoAttributeValue + 'static,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T, S> IntoDynNode for ReadSignal<T, S>
where
    T: Clone + IntoDynNode + 'static,
    S: BoxedSignalStorage<T>,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self.with(|f| f.clone().into_dyn_node())
    }
}

impl<T: Clone + 'static, S: BoxedSignalStorage<T>> Deref for ReadSignal<T, S> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<T: ?Sized, S: BoxedSignalStorage<T>> Readable for ReadSignal<T, S> {
    type Target = T;
    type Storage = S;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>
    where
        T: 'static,
    {
        if let Some(reactive_context) = ReactiveContext::current() {
            reactive_context.subscribe(self.ensure_subscribers());
            let forwarding_context = self.ensure_forwarding_context();
            forwarding_context.reset_and_run_in(|| {
                self.inner
                    .try_peek_unchecked()?
                    .value
                    .as_ref()
                    .expect("ReadSignal is missing its wrapped value")
                    .try_read_unchecked()
            })
        } else {
            self.inner
                .try_peek_unchecked()?
                .value
                .as_ref()
                .expect("ReadSignal is missing its wrapped value")
                .try_read_unchecked()
        }
    }

    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>>
    where
        T: 'static,
    {
        self.inner
            .try_peek_unchecked()?
            .value
            .as_ref()
            .expect("ReadSignal is missing its wrapped value")
            .try_peek_unchecked()
    }

    fn subscribers(&self) -> Subscribers
    where
        T: 'static,
    {
        self.ensure_subscribers()
    }
}

// We can't implement From<impl Readable<Target = T, Storage = S> > for ReadSignal<T, S>
// because it would conflict with the From<T> for T implementation, but we can implement it for
// all specific readable types
impl<
    T: 'static,
    S: CreateBoxedSignalStorage<Signal<T, S>> + BoxedSignalStorage<T> + Storage<SignalData<T>>,
> From<Signal<T, S>> for ReadSignal<T, S>
{
    fn from(value: Signal<T, S>) -> Self {
        Self::new_maybe_sync(value)
    }
}
impl<T: PartialEq + 'static> From<Memo<T>> for ReadSignal<T> {
    fn from(value: Memo<T>) -> Self {
        Self::new(value)
    }
}
impl<T: 'static, S: CreateBoxedSignalStorage<CopyValue<T, S>> + BoxedSignalStorage<T> + Storage<T>>
    From<CopyValue<T, S>> for ReadSignal<T, S>
{
    fn from(value: CopyValue<T, S>) -> Self {
        Self::new_maybe_sync(value)
    }
}
impl<T, R> From<Global<T, R>> for ReadSignal<R>
where
    T: Readable<Target = R, Storage = UnsyncStorage> + InitializeFromFunction<R> + Clone + 'static,
    R: 'static,
{
    fn from(value: Global<T, R>) -> Self {
        Self::new(value)
    }
}
impl<V, O, F, S> From<MappedSignal<O, V, F>> for ReadSignal<O, S>
where
    O: ?Sized + 'static,
    V: Readable<Storage = S> + 'static,
    F: Fn(&V::Target) -> &O + 'static,
    S: BoxedSignalStorage<O> + CreateBoxedSignalStorage<MappedSignal<O, V, F>>,
{
    fn from(value: MappedSignal<O, V, F>) -> Self {
        Self::new_maybe_sync(value)
    }
}
impl<V, O, F, FMut, S> From<MappedMutSignal<O, V, F, FMut>> for ReadSignal<O, S>
where
    O: ?Sized + 'static,
    V: Readable<Storage = S> + 'static,
    F: Fn(&V::Target) -> &O + 'static,
    FMut: 'static,
    S: BoxedSignalStorage<O> + CreateBoxedSignalStorage<MappedMutSignal<O, V, F, FMut>>,
{
    fn from(value: MappedMutSignal<O, V, F, FMut>) -> Self {
        Self::new_maybe_sync(value)
    }
}
impl<T: ?Sized + 'static, S> From<WriteSignal<T, S>> for ReadSignal<T, S>
where
    S: BoxedSignalStorage<T> + CreateBoxedSignalStorage<WriteSignal<T, S>>,
{
    fn from(value: WriteSignal<T, S>) -> Self {
        Self::new_maybe_sync(value)
    }
}

/// A boxed version of [Writable] that can be used to store any writable type.
pub struct WriteSignal<T: ?Sized, S: BoxedSignalStorage<T> = UnsyncStorage> {
    value: CopyValue<Box<S::DynWritable<sealed::SealedToken>>, S>,
}

impl<T: ?Sized + 'static> WriteSignal<T> {
    /// Create a new boxed writable value.
    pub fn new(
        value: impl Writable<Target = T, Storage = UnsyncStorage, WriteMetadata: 'static> + 'static,
    ) -> Self {
        Self::new_maybe_sync(value)
    }
}

impl<T: ?Sized + 'static, S: BoxedSignalStorage<T>> WriteSignal<T, S> {
    /// Create a new boxed writable value which may be sync
    pub fn new_maybe_sync<R>(value: R) -> Self
    where
        R: Writable<Target = T, WriteMetadata: 'static>,
        S: CreateBoxedSignalStorage<R>,
    {
        Self {
            value: CopyValue::new_maybe_sync(S::new_writable(value, sealed::SealedToken)),
        }
    }
}

struct BoxWriteMetadata<W> {
    value: W,
}

impl<W: Writable> BoxWriteMetadata<W> {
    fn new(value: W) -> Self {
        Self { value }
    }
}

impl<W: Readable> Readable for BoxWriteMetadata<W> {
    type Target = W::Target;

    type Storage = W::Storage;

    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>
    where
        W::Target: 'static,
    {
        self.value.try_read_unchecked()
    }

    fn try_peek_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>
    where
        W::Target: 'static,
    {
        self.value.try_peek_unchecked()
    }

    fn subscribers(&self) -> Subscribers
    where
        W::Target: 'static,
    {
        self.value.subscribers()
    }
}

impl<W> Writable for BoxWriteMetadata<W>
where
    W: Writable,
    W::WriteMetadata: 'static,
{
    type WriteMetadata = Box<dyn Any>;

    fn try_write_unchecked(
        &self,
    ) -> Result<crate::WritableRef<'static, Self>, generational_box::BorrowMutError>
    where
        W::Target: 'static,
    {
        self.value
            .try_write_unchecked()
            .map(|w| w.map_metadata(|data| Box::new(data) as Box<dyn Any>))
    }
}

impl<T: ?Sized, S: BoxedSignalStorage<T>> Clone for WriteSignal<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized, S: BoxedSignalStorage<T>> Copy for WriteSignal<T, S> {}

impl<T: ?Sized, S: BoxedSignalStorage<T>> PartialEq for WriteSignal<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

read_impls!(WriteSignal<T, S: BoxedSignalStorage<T>>);
write_impls!(WriteSignal<T, S: BoxedSignalStorage<T>>);

impl<T, S> IntoAttributeValue for WriteSignal<T, S>
where
    T: Clone + IntoAttributeValue + 'static,
    S: BoxedSignalStorage<T>,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T, S> IntoDynNode for WriteSignal<T, S>
where
    T: Clone + IntoDynNode + 'static,
    S: BoxedSignalStorage<T>,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self.with(|f| f.clone().into_dyn_node())
    }
}

impl<T: Clone + 'static, S: BoxedSignalStorage<T>> Deref for WriteSignal<T, S> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<T: ?Sized, S: BoxedSignalStorage<T>> Readable for WriteSignal<T, S> {
    type Target = T;
    type Storage = S;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>
    where
        T: 'static,
    {
        self.value.try_peek_unchecked()?.try_read_unchecked()
    }

    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>>
    where
        T: 'static,
    {
        self.value.try_peek_unchecked()?.try_peek_unchecked()
    }

    fn subscribers(&self) -> Subscribers
    where
        T: 'static,
    {
        self.value.try_peek_unchecked().unwrap().subscribers()
    }
}

impl<T: ?Sized, S: BoxedSignalStorage<T>> Writable for WriteSignal<T, S> {
    type WriteMetadata = Box<dyn Any>;

    fn try_write_unchecked(
        &self,
    ) -> Result<crate::WritableRef<'static, Self>, generational_box::BorrowMutError>
    where
        T: 'static,
    {
        self.value.try_peek_unchecked()?.try_write_unchecked()
    }
}

// We can't implement From<impl Writable<Target = T, Storage = S>> for Write<T, S>
// because it would conflict with the From<T> for T implementation, but we can implement it for
// all specific readable types
impl<
    T: 'static,
    S: CreateBoxedSignalStorage<Signal<T, S>> + BoxedSignalStorage<T> + Storage<SignalData<T>>,
> From<Signal<T, S>> for WriteSignal<T, S>
{
    fn from(value: Signal<T, S>) -> Self {
        Self::new_maybe_sync(value)
    }
}
impl<T: 'static, S: CreateBoxedSignalStorage<CopyValue<T, S>> + BoxedSignalStorage<T> + Storage<T>>
    From<CopyValue<T, S>> for WriteSignal<T, S>
{
    fn from(value: CopyValue<T, S>) -> Self {
        Self::new_maybe_sync(value)
    }
}
impl<T, R> From<Global<T, R>> for WriteSignal<R>
where
    T: Writable<Target = R, Storage = UnsyncStorage> + InitializeFromFunction<R> + Clone + 'static,
    R: 'static,
{
    fn from(value: Global<T, R>) -> Self {
        Self::new(value)
    }
}
impl<V, O, F, FMut, S> From<MappedMutSignal<O, V, F, FMut>> for WriteSignal<O, S>
where
    O: ?Sized + 'static,
    V: Writable<Storage = S> + 'static,
    F: Fn(&V::Target) -> &O + 'static,
    FMut: Fn(&mut V::Target) -> &mut O + 'static,
    S: CreateBoxedSignalStorage<MappedMutSignal<O, V, F, FMut>> + BoxedSignalStorage<O>,
{
    fn from(value: MappedMutSignal<O, V, F, FMut>) -> Self {
        Self::new_maybe_sync(value)
    }
}

/// A trait for creating boxed readable and writable signals. This is implemented for
/// [UnsyncStorage] and [SyncStorage].
///
/// You may need to add this trait as a bound when you use [ReadSignal] or [WriteSignal] while
/// remaining generic over syncness.
pub trait BoxedSignalStorage<T: ?Sized>:
    Storage<Box<Self::DynReadable<sealed::SealedToken>>>
    + Storage<Box<Self::DynWritable<sealed::SealedToken>>>
    + Storage<ReadSignalInner<T, Self>>
    + sealed::Sealed
    + 'static
{
    // This is not a public api, and is sealed to prevent external usage and implementations
    #[doc(hidden)]
    type DynReadable<Seal: sealed::SealedTokenTrait>: Readable<Target = T, Storage = Self> + ?Sized;
    // This is not a public api, and is sealed to prevent external usage and implementations
    #[doc(hidden)]
    type DynWritable<Seal: sealed::SealedTokenTrait>: Writable<Target = T, Storage = Self, WriteMetadata = Box<dyn Any>>
        + ?Sized;
}

/// A trait for creating boxed readable and writable signals. This is implemented for
/// [UnsyncStorage] and [SyncStorage].
///
/// The storage type must implement `CreateReadOnlySignalStorage<T>` for every readable `T` type
/// to be used with `ReadSignal` and `WriteSignal`.
///
/// You may need to add this trait as a bound when you call [ReadSignal::new_maybe_sync] or
/// [WriteSignal::new_maybe_sync] while remaining generic over syncness.
pub trait CreateBoxedSignalStorage<T: Readable + ?Sized>:
    BoxedSignalStorage<T::Target> + 'static
{
    // This is not a public api, and is sealed to prevent external usage and implementations
    #[doc(hidden)]
    fn new_readable(
        value: T,
        _: sealed::SealedToken,
    ) -> Box<Self::DynReadable<sealed::SealedToken>>
    where
        T: Sized;

    // This is not a public api, and is sealed to prevent external usage and implementations
    #[doc(hidden)]
    fn new_writable(
        value: T,
        _: sealed::SealedToken,
    ) -> Box<Self::DynWritable<sealed::SealedToken>>
    where
        T: Writable + Sized;
}

impl<T: ?Sized + 'static> BoxedSignalStorage<T> for UnsyncStorage {
    type DynReadable<Seal: sealed::SealedTokenTrait> = dyn Readable<Target = T, Storage = Self>;
    type DynWritable<Seal: sealed::SealedTokenTrait> =
        dyn Writable<Target = T, Storage = Self, WriteMetadata = Box<dyn Any>>;
}

impl<T: Readable<Storage = UnsyncStorage> + ?Sized + 'static> CreateBoxedSignalStorage<T>
    for UnsyncStorage
{
    fn new_readable(value: T, _: sealed::SealedToken) -> Box<Self::DynReadable<sealed::SealedToken>>
    where
        T: Sized,
    {
        Box::new(value)
    }

    fn new_writable(value: T, _: sealed::SealedToken) -> Box<Self::DynWritable<sealed::SealedToken>>
    where
        T: Writable + Sized,
    {
        Box::new(BoxWriteMetadata::new(value))
    }
}

impl<T: ?Sized + 'static> BoxedSignalStorage<T> for SyncStorage {
    type DynReadable<Seal: sealed::SealedTokenTrait> =
        dyn Readable<Target = T, Storage = Self> + Send + Sync;
    type DynWritable<Seal: sealed::SealedTokenTrait> =
        dyn Writable<Target = T, Storage = Self, WriteMetadata = Box<dyn Any>> + Send + Sync;
}

impl<T: Readable<Storage = SyncStorage> + Sync + Send + ?Sized + 'static>
    CreateBoxedSignalStorage<T> for SyncStorage
{
    fn new_readable(value: T, _: sealed::SealedToken) -> Box<Self::DynReadable<sealed::SealedToken>>
    where
        T: Sized,
    {
        Box::new(value)
    }

    fn new_writable(value: T, _: sealed::SealedToken) -> Box<Self::DynWritable<sealed::SealedToken>>
    where
        T: Writable + Sized,
    {
        Box::new(BoxWriteMetadata::new(value))
    }
}

mod sealed {
    use generational_box::{SyncStorage, UnsyncStorage};

    pub trait Sealed {}
    impl Sealed for UnsyncStorage {}
    impl Sealed for SyncStorage {}

    pub struct SealedToken;

    pub trait SealedTokenTrait {}
    impl SealedTokenTrait for SealedToken {}
}
