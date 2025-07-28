use std::{any::Any, ops::Deref};

use dioxus_core::{IntoAttributeValue, IntoDynNode, Subscribers};
use generational_box::{BorrowResult, UnsyncStorage};

use crate::{
    read_impls, write_impls, CopyValue, Global, InitializeFromFunction, MappedMutSignal,
    MappedSignal, Memo, Readable, ReadableExt, ReadableRef, Signal, Writable, WritableExt,
};

/// A signal that can only be read from.
pub type ReadOnlySignal<T> = ReadSignal<T>;

/// A boxed version of [Readable] that can be used to store any readable type.
pub struct ReadSignal<T: ?Sized + 'static> {
    value: CopyValue<Box<dyn Readable<Target = T, Storage = UnsyncStorage>>>,
}

impl<T: ?Sized + 'static> ReadSignal<T> {
    /// Create a new boxed readable value.
    pub fn new(value: impl Readable<Target = T, Storage = UnsyncStorage> + 'static) -> Self {
        Self {
            value: CopyValue::new(Box::new(value)),
        }
    }

    /// Point to another [ReadSignal]. This will subscribe the other [ReadSignal] to all subscribers of this [ReadSignal].
    pub fn point_to(&self, other: Self) -> BorrowResult
    where
        T: Sized + 'static,
    {
        #[allow(clippy::mutable_key_type)]
        let this_subscribers = self.subscribers();
        let other_subscribers = other.subscribers();
        if let (Some(this_subscribers), Some(other_subscribers)) =
            (this_subscribers, other_subscribers)
        {
            this_subscribers.visit(|subscriber| {
                subscriber.subscribe(other_subscribers.clone());
            });
        }
        self.value.point_to(other.value)
    }

    #[doc(hidden)]
    /// This is only used by the `props` macro.
    /// Mark any readers of the signal as dirty
    pub fn mark_dirty(&mut self) {
        let subscribers = self.value.subscribers();
        if let Some(subscribers) = subscribers {
            subscribers.visit(|subscriber| {
                subscriber.mark_dirty();
            });
        }
    }
}

impl<T: ?Sized + 'static> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + 'static> Copy for ReadSignal<T> {}

impl<T: ?Sized + 'static> PartialEq for ReadSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Default + 'static> Default for ReadSignal<T> {
    fn default() -> Self {
        Self::new(Signal::new(T::default()))
    }
}

read_impls!(ReadSignal<T>);

impl<T> IntoAttributeValue for ReadSignal<T>
where
    T: Clone + IntoAttributeValue + 'static,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T> IntoDynNode for ReadSignal<T>
where
    T: Clone + IntoDynNode + 'static,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self.with(|f| f.clone().into_dyn_node())
    }
}

impl<T: Clone + 'static> Deref for ReadSignal<T> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<T: 'static> Readable for ReadSignal<T> {
    type Target = T;
    type Storage = UnsyncStorage;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.value
            .try_peek_unchecked()
            .unwrap()
            .try_read_unchecked()
    }

    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>> {
        self.value
            .try_peek_unchecked()
            .unwrap()
            .try_peek_unchecked()
    }

    fn subscribers(&self) -> Option<Subscribers> {
        self.value.subscribers()
    }
}

// We can't implement From<impl Readable<Target = T, Storage = S> + 'static> for ReadSignal<T, S>
// because it would conflict with the From<T> for T implementation, but we can implement it for
// all specific readable types
impl<T> From<Signal<T>> for ReadSignal<T> {
    fn from(value: Signal<T>) -> Self {
        Self::new(value)
    }
}
impl<T: PartialEq> From<Memo<T>> for ReadSignal<T> {
    fn from(value: Memo<T>) -> Self {
        Self::new(value)
    }
}
impl<T> From<CopyValue<T>> for ReadSignal<T> {
    fn from(value: CopyValue<T>) -> Self {
        Self::new(value)
    }
}
impl<T: Clone + 'static, R: 'static> From<Global<T, R>> for ReadSignal<R>
where
    T: Readable<Target = R, Storage = UnsyncStorage> + InitializeFromFunction<R>,
{
    fn from(value: Global<T, R>) -> Self {
        Self::new(value)
    }
}
impl<V, O, F> From<MappedSignal<O, V, F>> for ReadSignal<O>
where
    O: ?Sized,
    V: Readable<Storage = UnsyncStorage> + 'static,
    F: Fn(&V::Target) -> &O + 'static,
{
    fn from(value: MappedSignal<O, V, F>) -> Self {
        Self::new(value)
    }
}
impl<V, O, F, FMut> From<MappedMutSignal<O, V, F, FMut>> for ReadSignal<O>
where
    O: ?Sized,
    V: Readable<Storage = UnsyncStorage> + 'static,
    F: Fn(&V::Target) -> &O + 'static,
    FMut: 'static,
{
    fn from(value: MappedMutSignal<O, V, F, FMut>) -> Self {
        Self::new(value)
    }
}

/// A boxed version of [Writable] that can be used to store any writable type.
pub struct WriteSignal<T: ?Sized + 'static> {
    value: CopyValue<
        Box<dyn Writable<Target = T, Storage = UnsyncStorage, WriteMetadata = Box<dyn Any>>>,
    >,
}

impl<T: ?Sized> WriteSignal<T> {
    /// Create a new boxed writable value.
    pub fn new<M>(
        value: impl Writable<Target = T, Storage = UnsyncStorage, WriteMetadata = M> + 'static,
    ) -> Self {
        Self {
            value: CopyValue::new(Box::new(BoxWriteMetadata::new(value))),
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
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.value.try_read_unchecked()
    }

    fn try_peek_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.value.try_peek_unchecked()
    }

    fn subscribers(&self) -> Option<Subscribers> {
        self.value.subscribers()
    }
}

impl<W: Writable> Writable for BoxWriteMetadata<W> {
    type WriteMetadata = Box<dyn Any>;

    fn try_write_unchecked(
        &self,
    ) -> Result<crate::WritableRef<'static, Self>, generational_box::BorrowMutError> {
        self.value
            .try_write_unchecked()
            .map(|w| w.map_metadata(|data| Box::new(data) as Box<dyn Any>))
    }

    fn write(&mut self) -> crate::WritableRef<'_, Self> {
        self.value
            .write()
            .map_metadata(|data| Box::new(data) as Box<dyn Any>)
    }

    fn try_write(
        &mut self,
    ) -> Result<crate::WritableRef<'_, Self>, generational_box::BorrowMutError> {
        self.value
            .try_write()
            .map(|w| w.map_metadata(|data| Box::new(data) as Box<dyn Any>))
    }

    fn write_unchecked(&self) -> crate::WritableRef<'static, Self> {
        self.value
            .write_unchecked()
            .map_metadata(|data| Box::new(data) as Box<dyn Any>)
    }
}

impl<T: ?Sized> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for WriteSignal<T> {}

impl<T: ?Sized> PartialEq for WriteSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

read_impls!(WriteSignal<T>);
write_impls!(WriteSignal<T>);

impl<T> IntoAttributeValue for WriteSignal<T>
where
    T: Clone + IntoAttributeValue + 'static,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T> IntoDynNode for WriteSignal<T>
where
    T: Clone + IntoDynNode + 'static,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self.with(|f| f.clone().into_dyn_node())
    }
}

impl<T: Clone + 'static> Deref for WriteSignal<T> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<T: 'static> Readable for WriteSignal<T> {
    type Target = T;
    type Storage = UnsyncStorage;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.value
            .try_peek_unchecked()
            .unwrap()
            .try_read_unchecked()
    }

    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>> {
        self.value
            .try_peek_unchecked()
            .unwrap()
            .try_peek_unchecked()
    }

    fn subscribers(&self) -> Option<Subscribers> {
        self.value.subscribers()
    }
}

impl<T: 'static> Writable for WriteSignal<T> {
    type WriteMetadata = Box<dyn Any>;

    fn write_unchecked(&self) -> crate::WritableRef<'static, Self> {
        self.value.try_peek_unchecked().unwrap().write_unchecked()
    }

    fn try_write_unchecked(
        &self,
    ) -> Result<crate::WritableRef<'static, Self>, generational_box::BorrowMutError> {
        self.value
            .try_peek_unchecked()
            .unwrap()
            .try_write_unchecked()
    }
}

// We can't implement From<impl Writable<Target = T, Storage = S> + 'static> for Write<T, S>
// because it would conflict with the From<T> for T implementation, but we can implement it for
// all specific readable types
impl<T> From<Signal<T>> for WriteSignal<T> {
    fn from(value: Signal<T>) -> Self {
        Self::new(value)
    }
}
impl<T> From<CopyValue<T>> for WriteSignal<T> {
    fn from(value: CopyValue<T>) -> Self {
        Self::new(value)
    }
}
impl<T: Clone + 'static, R: 'static> From<Global<T, R>> for WriteSignal<R>
where
    T: Writable<Target = R, Storage = UnsyncStorage> + InitializeFromFunction<R>,
{
    fn from(value: Global<T, R>) -> Self {
        Self::new(value)
    }
}
impl<V, O, F, FMut> From<MappedMutSignal<O, V, F, FMut>> for WriteSignal<O>
where
    O: ?Sized,
    V: Writable<Storage = UnsyncStorage> + 'static,
    F: Fn(&V::Target) -> &O + 'static,
    FMut: Fn(&mut V::Target) -> &mut O + 'static,
{
    fn from(value: MappedMutSignal<O, V, F, FMut>) -> Self {
        Self::new(value)
    }
}
