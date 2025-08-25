use std::{any::Any, ops::Deref};

use dioxus_core::{IntoAttributeValue, IntoDynNode, Subscribers};
use generational_box::{BorrowResult, Storage, SyncStorage, UnsyncStorage};

use crate::{
    read_impls, write_impls, CopyValue, Global, InitializeFromFunction, MappedMutSignal,
    MappedSignal, Memo, Readable, ReadableExt, ReadableRef, Signal, SignalData, Writable,
    WritableExt,
};

/// A signal that can only be read from.
#[deprecated(
    since = "0.7.0",
    note = "Use `ReadSignal` instead. Will be removed in 0.8"
)]
pub type ReadOnlySignal<T, S> = ReadSignal<T, S>;

mod sealed {
    use generational_box::{SyncStorage, UnsyncStorage};

    pub trait Sealed {}
    impl Sealed for UnsyncStorage {}
    impl Sealed for SyncStorage {}

    pub struct SealedToken;
}

/// A trait for creating boxed readable and writable signals. This is implemented for
/// [UnsyncStorage] and [SyncStorage].
pub trait ReadOnlySignalStorage<T: ?Sized>:
    Storage<Box<Self::DynReadable>> + Storage<Box<Self::DynWritable>> + sealed::Sealed + 'static
{
    #[doc(hidden)]
    type DynReadable: Readable<Target = T, Storage = Self> + ?Sized;
    #[doc(hidden)]
    type DynWritable: Writable<Target = T, Storage = Self, WriteMetadata = Box<dyn Any>> + ?Sized;
}

/// A trait for creating boxed readable and writable signals. This is implemented for
/// [UnsyncStorage] and [SyncStorage].
///
/// The storage type must implement `CreateReadOnlySignalStorage` for the specific readable type
/// to be used with `ReadSignal` and `WriteSignal`.
pub trait CreateReadOnlySignalStorage<T: Readable + ?Sized>:
    ReadOnlySignalStorage<T::Target> + 'static
{
    #[doc(hidden)]
    fn new_readable(value: T, _: sealed::SealedToken) -> Box<Self::DynReadable>
    where
        T: Sized;

    #[doc(hidden)]
    fn new_writable(value: T, _: sealed::SealedToken) -> Box<Self::DynWritable>
    where
        T: Writable + Sized;
}

impl<T: ?Sized + 'static> ReadOnlySignalStorage<T> for UnsyncStorage {
    type DynReadable = dyn Readable<Target = T, Storage = Self>;
    type DynWritable = dyn Writable<Target = T, Storage = Self, WriteMetadata = Box<dyn Any>>;
}

impl<T: Readable<Storage = UnsyncStorage> + ?Sized + 'static> CreateReadOnlySignalStorage<T>
    for UnsyncStorage
{
    fn new_readable(value: T, _: sealed::SealedToken) -> Box<Self::DynReadable>
    where
        T: Sized,
    {
        Box::new(value)
    }

    fn new_writable(value: T, _: sealed::SealedToken) -> Box<Self::DynWritable>
    where
        T: Writable + Sized,
    {
        Box::new(BoxWriteMetadata::new(value))
    }
}

impl<T: ?Sized + 'static> ReadOnlySignalStorage<T> for SyncStorage {
    type DynReadable = dyn Readable<Target = T, Storage = Self> + Send + Sync;
    type DynWritable =
        dyn Writable<Target = T, Storage = Self, WriteMetadata = Box<dyn Any>> + Send + Sync;
}

impl<T: Readable<Storage = SyncStorage> + Sync + Send + ?Sized + 'static>
    CreateReadOnlySignalStorage<T> for SyncStorage
{
    fn new_readable(value: T, _: sealed::SealedToken) -> Box<Self::DynReadable>
    where
        T: Sized,
    {
        Box::new(value)
    }

    fn new_writable(value: T, _: sealed::SealedToken) -> Box<Self::DynWritable>
    where
        T: Writable + Sized,
    {
        Box::new(BoxWriteMetadata::new(value))
    }
}

/// A boxed version of [Readable] that can be used to store any readable type.
pub struct ReadSignal<T: ?Sized, S: ReadOnlySignalStorage<T> = UnsyncStorage> {
    value: CopyValue<Box<S::DynReadable>, S>,
}

impl<T: ?Sized + 'static> ReadSignal<T> {
    /// Create a new boxed readable value.
    pub fn new(value: impl Readable<Target = T, Storage = UnsyncStorage> + 'static) -> Self {
        Self::new_maybe_sync(value)
    }
}

impl<T: ?Sized + 'static, S: ReadOnlySignalStorage<T>> ReadSignal<T, S> {
    /// Create a new boxed readable value which may be sync
    pub fn new_maybe_sync<R>(value: R) -> Self
    where
        S: CreateReadOnlySignalStorage<R>,
        R: Readable<Target = T>,
    {
        Self {
            value: CopyValue::new_maybe_sync(S::new_readable(value, sealed::SealedToken)),
        }
    }

    /// Point to another [ReadSignal]. This will subscribe the other [ReadSignal] to all subscribers of this [ReadSignal].
    pub fn point_to(&self, other: Self) -> BorrowResult {
        let this_subscribers = self.subscribers();
        let mut this_subscribers_vec = Vec::new();
        // Note we don't subscribe directly in the visit closure to avoid a deadlock when pointing to self
        this_subscribers.visit(|subscriber| this_subscribers_vec.push(*subscriber));
        let other_subscribers = other.subscribers();
        for subscriber in this_subscribers_vec {
            subscriber.subscribe(other_subscribers.clone());
        }
        self.value.point_to(other.value)?;
        Ok(())
    }

    #[doc(hidden)]
    /// This is only used by the `props` macro.
    /// Mark any readers of the signal as dirty
    pub fn mark_dirty(&mut self) {
        let subscribers = self.subscribers();
        let mut this_subscribers_vec = Vec::new();
        subscribers.visit(|subscriber| this_subscribers_vec.push(*subscriber));
        for subscriber in this_subscribers_vec {
            subscribers.remove(&subscriber);
            subscriber.mark_dirty();
        }
    }
}

impl<T: ?Sized, S: ReadOnlySignalStorage<T>> Clone for ReadSignal<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized, S: ReadOnlySignalStorage<T>> Copy for ReadSignal<T, S> {}

impl<T: ?Sized, S: ReadOnlySignalStorage<T>> PartialEq for ReadSignal<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<
        T: Default + 'static,
        S: CreateReadOnlySignalStorage<Signal<T, S>>
            + ReadOnlySignalStorage<T>
            + Storage<SignalData<T>>,
    > Default for ReadSignal<T, S>
{
    fn default() -> Self {
        Self::new_maybe_sync(Signal::new_maybe_sync(T::default()))
    }
}

read_impls!(ReadSignal<T, S: ReadOnlySignalStorage<T>>);

impl<T, S: ReadOnlySignalStorage<T>> IntoAttributeValue for ReadSignal<T, S>
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
    S: ReadOnlySignalStorage<T>,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self.with(|f| f.clone().into_dyn_node())
    }
}

impl<T: Clone + 'static, S: ReadOnlySignalStorage<T>> Deref for ReadSignal<T, S> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<T: ?Sized, S: ReadOnlySignalStorage<T>> Readable for ReadSignal<T, S> {
    type Target = T;
    type Storage = S;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>
    where
        T: 'static,
    {
        self.value
            .try_peek_unchecked()
            .unwrap()
            .try_read_unchecked()
    }

    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>>
    where
        T: 'static,
    {
        self.value
            .try_peek_unchecked()
            .unwrap()
            .try_peek_unchecked()
    }

    fn subscribers(&self) -> Subscribers
    where
        T: 'static,
    {
        self.value.try_peek_unchecked().unwrap().subscribers()
    }
}

// We can't implement From<impl Readable<Target = T, Storage = S> > for ReadSignal<T, S>
// because it would conflict with the From<T> for T implementation, but we can implement it for
// all specific readable types
impl<
        T: 'static,
        S: CreateReadOnlySignalStorage<Signal<T, S>>
            + ReadOnlySignalStorage<T>
            + Storage<SignalData<T>>,
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
impl<
        T: 'static,
        S: CreateReadOnlySignalStorage<CopyValue<T, S>> + ReadOnlySignalStorage<T> + Storage<T>,
    > From<CopyValue<T, S>> for ReadSignal<T, S>
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
    S: ReadOnlySignalStorage<O> + CreateReadOnlySignalStorage<MappedSignal<O, V, F>>,
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
    S: ReadOnlySignalStorage<O> + CreateReadOnlySignalStorage<MappedMutSignal<O, V, F, FMut>>,
{
    fn from(value: MappedMutSignal<O, V, F, FMut>) -> Self {
        Self::new_maybe_sync(value)
    }
}
impl<T: ?Sized + 'static, S> From<WriteSignal<T, S>> for ReadSignal<T, S>
where
    S: ReadOnlySignalStorage<T> + CreateReadOnlySignalStorage<WriteSignal<T, S>>,
{
    fn from(value: WriteSignal<T, S>) -> Self {
        Self::new_maybe_sync(value)
    }
}

/// A boxed version of [Writable] that can be used to store any writable type.
pub struct WriteSignal<T: ?Sized, S: ReadOnlySignalStorage<T> = UnsyncStorage> {
    value: CopyValue<Box<S::DynWritable>, S>,
}

impl<T: ?Sized + 'static> WriteSignal<T> {
    /// Create a new boxed writable value.
    pub fn new(
        value: impl Writable<Target = T, Storage = UnsyncStorage, WriteMetadata: 'static> + 'static,
    ) -> Self {
        Self::new_maybe_sync(value)
    }
}

impl<T: ?Sized + 'static, S: ReadOnlySignalStorage<T>> WriteSignal<T, S> {
    /// Create a new boxed writable value which may be sync
    pub fn new_maybe_sync<R>(value: R) -> Self
    where
        R: Writable<Target = T, WriteMetadata: 'static>,
        S: CreateReadOnlySignalStorage<R>,
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

impl<T: ?Sized, S: ReadOnlySignalStorage<T>> Clone for WriteSignal<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized, S: ReadOnlySignalStorage<T>> Copy for WriteSignal<T, S> {}

impl<T: ?Sized, S: ReadOnlySignalStorage<T>> PartialEq for WriteSignal<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

read_impls!(WriteSignal<T, S: ReadOnlySignalStorage<T>>);
write_impls!(WriteSignal<T, S: ReadOnlySignalStorage<T>>);

impl<T, S> IntoAttributeValue for WriteSignal<T, S>
where
    T: Clone + IntoAttributeValue + 'static,
    S: ReadOnlySignalStorage<T>,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T, S> IntoDynNode for WriteSignal<T, S>
where
    T: Clone + IntoDynNode + 'static,
    S: ReadOnlySignalStorage<T>,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self.with(|f| f.clone().into_dyn_node())
    }
}

impl<T: Clone + 'static, S: ReadOnlySignalStorage<T>> Deref for WriteSignal<T, S> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<T: ?Sized, S: ReadOnlySignalStorage<T>> Readable for WriteSignal<T, S> {
    type Target = T;
    type Storage = S;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>
    where
        T: 'static,
    {
        self.value
            .try_peek_unchecked()
            .unwrap()
            .try_read_unchecked()
    }

    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>>
    where
        T: 'static,
    {
        self.value
            .try_peek_unchecked()
            .unwrap()
            .try_peek_unchecked()
    }

    fn subscribers(&self) -> Subscribers
    where
        T: 'static,
    {
        self.value.try_peek_unchecked().unwrap().subscribers()
    }
}

impl<T: ?Sized, S: ReadOnlySignalStorage<T>> Writable for WriteSignal<T, S> {
    type WriteMetadata = Box<dyn Any>;

    fn try_write_unchecked(
        &self,
    ) -> Result<crate::WritableRef<'static, Self>, generational_box::BorrowMutError>
    where
        T: 'static,
    {
        self.value
            .try_peek_unchecked()
            .unwrap()
            .try_write_unchecked()
    }
}

// We can't implement From<impl Writable<Target = T, Storage = S>> for Write<T, S>
// because it would conflict with the From<T> for T implementation, but we can implement it for
// all specific readable types
impl<
        T: 'static,
        S: CreateReadOnlySignalStorage<Signal<T, S>>
            + ReadOnlySignalStorage<T>
            + Storage<SignalData<T>>,
    > From<Signal<T, S>> for WriteSignal<T, S>
{
    fn from(value: Signal<T, S>) -> Self {
        Self::new_maybe_sync(value)
    }
}
impl<
        T: 'static,
        S: CreateReadOnlySignalStorage<CopyValue<T, S>> + ReadOnlySignalStorage<T> + Storage<T>,
    > From<CopyValue<T, S>> for WriteSignal<T, S>
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
    S: CreateReadOnlySignalStorage<MappedMutSignal<O, V, F, FMut>> + ReadOnlySignalStorage<O>,
{
    fn from(value: MappedMutSignal<O, V, F, FMut>) -> Self {
        Self::new_maybe_sync(value)
    }
}
