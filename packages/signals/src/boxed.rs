use std::{any::Any, ops::Deref};

use dioxus_core::{
    IntoAttributeValue, IntoDynNode, ReactiveContext, Subscribers, current_scope_id,
};
use generational_box::{BorrowResult, Owner, Storage, SyncStorage, UnsyncStorage};

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

/// Wrapper subscriber state plus a reactive context that forwards updates from the current
/// readable source.
///
/// `ReadSignal` is a reactive proxy. A child component subscribes to this wrapper, and the
/// forwarding context subscribes to the readable source the wrapper currently points at. `point_to`
/// retargets only that source subscription; wrapper subscribers stay attached to this
/// `ForwardingContext`.
///
/// Running source reads under this context preserves normal `Readable` subscription behavior for
/// signals, stores, and memos. When the source changes, this context marks wrapper subscribers
/// dirty without moving or clearing direct subscriptions made outside the wrapper.
///
/// # Example
///
/// ```rust,ignore
/// fn app() -> Element {
///     let mut use_b = use_signal(|| false);
///     let signal_a = use_signal(|| 0);
///     let signal_b = use_signal(|| 0);
///
///     use_effect(move || {
///         signal_a();
///         // This effect's context subscribes directly to signal_a. It is not a
///         // wrapper-level `ReadSignal` subscriber, so retargeting a child prop
///         // must not move this subscription to signal_b.
///     });
///
///     let child_signal = if use_b() { signal_b } else { signal_a };
///     // When signal_a and signal_b currently hold equal values, props can be
///     // memoized in place:
///     //
///     // old ReadSignal(signal_a).point_to(new ReadSignal(signal_b))
///     //
///     // That swap should keep wrapper subscribers attached to the existing
///     // wrapper and retarget only its source subscription.
///     rsx! { Child { sig: child_signal } }
/// }
///
/// #[component]
/// fn Child(sig: ReadSignal<i32>) -> Element {
///     rsx! {
///         // This read subscribes the child to the ReadSignal wrapper. The
///         // wrapper's forwarding context subscribes to the current source.
///         "{sig}"
///     }
/// }
/// ```
#[doc(hidden)]
pub struct ForwardingContext {
    subscribers: Subscribers,
    forwarding_context: ReactiveContext,
    _owner: Owner<SyncStorage>,
}

impl ForwardingContext {
    fn new(wrapped_subscribers: Subscribers) -> Self {
        let subscribers = Subscribers::new();
        let subscribers_to_notify = subscribers.clone();
        let owner = Owner::<SyncStorage>::default();
        let forwarding_context = ReactiveContext::new_with_callback_in_owner(
            move || mark_subscribers_dirty(&subscribers_to_notify),
            current_scope_id(),
            owner.clone(),
            std::panic::Location::caller(),
        );
        forwarding_context.subscribe(wrapped_subscribers);

        Self {
            subscribers,
            forwarding_context,
            _owner: owner,
        }
    }

    fn subscribers(&self) -> Subscribers {
        self.subscribers.clone()
    }

    fn run_in<O>(&self, f: impl FnOnce() -> O) -> O {
        self.forwarding_context.run_in(f)
    }

    fn retarget_source(&self, wrapped_subscribers: Subscribers) {
        self.forwarding_context.clear_subscribers();
        self.forwarding_context.subscribe(wrapped_subscribers);
    }

    fn mark_dirty(&self) {
        mark_subscribers_dirty(&self.subscribers);
    }
}

impl Drop for ForwardingContext {
    fn drop(&mut self) {
        self.forwarding_context.clear_subscribers();
    }
}

fn mark_subscribers_dirty(subscribers: &Subscribers) {
    let mut subscribers_to_notify = Vec::new();
    subscribers.visit(|subscriber| subscribers_to_notify.push(*subscriber));
    for subscriber in subscribers_to_notify {
        if !subscriber.mark_dirty() {
            subscribers.remove(&subscriber);
        }
    }
}

/// A boxed version of [Readable] that can be used to store any readable type.
pub struct ReadSignal<T: ?Sized, S: BoxedSignalStorage<T> = UnsyncStorage> {
    value: CopyValue<Box<S::DynReadable<sealed::SealedToken>>, S>,
    forwarding: CopyValue<ForwardingContext, S>,
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
        let value = S::new_readable(value, sealed::SealedToken);
        let subscribers = ForwardingContext::new(value.subscribers());
        Self {
            value: CopyValue::new_maybe_sync(value),
            forwarding: CopyValue::new_maybe_sync(subscribers),
        }
    }

    /// Point to another [ReadSignal]. Wrapper-level subscribers stay attached to this wrapper;
    /// subscribers attached directly to the underlying readable are left alone.
    pub fn point_to(&self, other: Self) -> BorrowResult {
        if self.forwarding == other.forwarding {
            return Ok(());
        }

        let forwarding = match self.forwarding.try_peek_unchecked() {
            Ok(forwarding) => forwarding,
            Err(_) => return Ok(()),
        };

        let new_value = other.value;
        let new_source_subscribers = new_value.try_peek_unchecked()?.subscribers();

        // Keep `other` usable; rsx clones can retarget multiple props from it.
        self.value.point_to(new_value)?;

        forwarding.retarget_source(new_source_subscribers);
        Ok(())
    }

    #[doc(hidden)]
    /// This is only used by the `props` macro.
    /// Mark any readers of the signal as dirty
    pub fn mark_dirty(&mut self) {
        let forwarding = self.forwarding.try_peek_unchecked().unwrap();
        forwarding.mark_dirty();
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
        self.forwarding == other.forwarding
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
        let forwarding = self.forwarding.try_peek_unchecked()?;
        let wrapped = self.value.try_peek_unchecked()?;
        if let Some(reactive_context) = ReactiveContext::current() {
            reactive_context.subscribe(forwarding.subscribers());
        }
        forwarding.run_in(|| wrapped.try_read_unchecked())
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
        let forwarding = self.forwarding.try_peek_unchecked().unwrap();
        forwarding.subscribers()
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
    + Storage<ForwardingContext>
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
