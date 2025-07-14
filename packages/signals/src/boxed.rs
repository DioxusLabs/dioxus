use std::{ops::Deref, rc::Rc};

use dioxus_core::{prelude::IntoAttributeValue, IntoDynNode};
use generational_box::{BorrowResult, Storage, UnsyncStorage};

use crate::{
    read_impls, write_impls, CopyValue, Global, InitializeFromFunction, MappedMutSignal,
    MappedSignal, Memo, ReadOnlySignal, Readable, ReadableExt, ReadableRef, Signal, SignalData,
    SignalWriteStorage, Writable, WritableExt, WriteRefStorage,
};

/// A boxed version of [Readable] that can be used to store any readable type.
pub struct BoxedReadable<T: ?Sized, S: ?Sized = UnsyncStorage> {
    value: Rc<dyn Readable<Target = T, Storage = S>>,
}

impl<T: ?Sized, S: ?Sized> BoxedReadable<T, S> {
    /// Create a new boxed readable value.
    pub fn new(value: impl Readable<Target = T, Storage = S> + 'static) -> Self {
        Self {
            value: Rc::new(value),
        }
    }
}

impl<T: ?Sized, S: ?Sized> Clone for BoxedReadable<T, S> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

impl<T: ?Sized, S: ?Sized> PartialEq for BoxedReadable<T, S> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.value, &other.value)
    }
}

read_impls!(BoxedReadable<T, S> where S: Storage<T>);

impl<T, S> IntoAttributeValue for BoxedReadable<T, S>
where
    T: Clone + IntoAttributeValue + 'static,
    S: Storage<T>,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T, S> IntoDynNode for BoxedReadable<T, S>
where
    T: Clone + IntoDynNode + 'static,
    S: Storage<T>,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self.with(|f| f.clone().into_dyn_node())
    }
}

impl<T: Clone + 'static, S: Storage<T>> Deref for BoxedReadable<T, S> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<T: 'static, S: Storage<T>> Readable for BoxedReadable<T, S> {
    type Target = T;
    type Storage = S;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.value.try_read_unchecked()
    }

    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>> {
        self.value.try_peek_unchecked()
    }

    fn read(&self) -> ReadableRef<Self> {
        self.value.read()
    }

    fn try_read(&self) -> Result<ReadableRef<Self>, generational_box::BorrowError> {
        self.value.try_read()
    }

    fn read_unchecked(&self) -> ReadableRef<'static, Self> {
        self.value.read_unchecked()
    }

    fn peek(&self) -> ReadableRef<Self> {
        self.value.peek()
    }

    fn try_peek(&self) -> Result<ReadableRef<Self>, generational_box::BorrowError> {
        self.value.try_peek()
    }

    fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
        self.value.peek_unchecked()
    }
}

// We can't implement From<impl Readable<Target = T, Storage = S> + 'static> for BoxedReadable<T, S>
// because it would conflict with the From<T> for T implementation, but we can implement it for
// all specific readable types
impl<T, S: Storage<SignalData<T>>> From<Signal<T, S>> for BoxedReadable<T, S> {
    fn from(value: Signal<T, S>) -> Self {
        Self::new(value)
    }
}
impl<T, S: Storage<SignalData<T>>> From<ReadOnlySignal<T, S>> for BoxedReadable<T, S> {
    fn from(value: ReadOnlySignal<T, S>) -> Self {
        Self::new(value)
    }
}
impl<T: PartialEq> From<Memo<T>> for BoxedReadable<T> {
    fn from(value: Memo<T>) -> Self {
        Self::new(value)
    }
}
impl<T, S: Storage<T>> From<CopyValue<T, S>> for BoxedReadable<T, S> {
    fn from(value: CopyValue<T, S>) -> Self {
        Self::new(value)
    }
}
impl<T: Clone + 'static, S, R: 'static> From<Global<T, R>> for BoxedReadable<R, S>
where
    T: Readable<Target = R, Storage = S> + InitializeFromFunction<R>,
{
    fn from(value: Global<T, R>) -> Self {
        Self::new(value)
    }
}
impl<V, O, F> From<MappedSignal<O, V, F>> for BoxedReadable<O, V::Storage>
where
    O: ?Sized,
    V: Readable + 'static,
    F: Fn(&V::Target) -> &O + 'static,
{
    fn from(value: MappedSignal<O, V, F>) -> Self {
        Self::new(value)
    }
}
impl<V, O, F, FMut> From<MappedMutSignal<O, V, F, FMut>> for BoxedReadable<O, V::Storage>
where
    O: ?Sized,
    V: Readable + 'static,
    F: Fn(&V::Target) -> &O + 'static,
    FMut: 'static,
{
    fn from(value: MappedMutSignal<O, V, F, FMut>) -> Self {
        Self::new(value)
    }
}

/// A boxed version of [Writable] that can be used to store any writable type.
pub struct BoxedWritable<T: ?Sized, S: ?Sized = UnsyncStorage, M: ?Sized = S> {
    value: Rc<dyn Writable<Target = T, Storage = S, Mut = M>>,
}

impl<T: ?Sized, S: ?Sized, M: ?Sized> BoxedWritable<T, S, M> {
    /// Create a new boxed writable value.
    pub fn new(value: impl Writable<Target = T, Storage = S, Mut = M> + 'static) -> Self {
        Self {
            value: Rc::new(value),
        }
    }
}

impl<T: ?Sized, S: ?Sized, M: ?Sized> Clone for BoxedWritable<T, S, M> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

impl<T: ?Sized, S: ?Sized, M: ?Sized> PartialEq for BoxedWritable<T, S, M> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.value, &other.value)
    }
}

read_impls!(BoxedWritable<T, S, M> where S: Storage<T>);
write_impls!(BoxedWritable<T, S, M> where S: Storage<T>, M: WriteRefStorage);

impl<T, S, M> IntoAttributeValue for BoxedWritable<T, S, M>
where
    T: Clone + IntoAttributeValue + 'static,
    S: Storage<T>,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T, S, M> IntoDynNode for BoxedWritable<T, S, M>
where
    T: Clone + IntoDynNode + 'static,
    S: Storage<T>,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self.with(|f| f.clone().into_dyn_node())
    }
}

impl<T: Clone + 'static, S: Storage<T>, M: 'static> Deref for BoxedWritable<T, S, M> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<T: 'static, S: Storage<T>, M> Readable for BoxedWritable<T, S, M> {
    type Target = T;
    type Storage = S;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.value.try_read_unchecked()
    }

    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>> {
        self.value.try_peek_unchecked()
    }

    fn read(&self) -> ReadableRef<Self> {
        self.value.read()
    }

    fn try_read(&self) -> Result<ReadableRef<Self>, generational_box::BorrowError> {
        self.value.try_read()
    }

    fn read_unchecked(&self) -> ReadableRef<'static, Self> {
        self.value.read_unchecked()
    }

    fn peek(&self) -> ReadableRef<Self> {
        self.value.peek()
    }

    fn try_peek(&self) -> Result<ReadableRef<Self>, generational_box::BorrowError> {
        self.value.try_peek()
    }

    fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
        self.value.peek_unchecked()
    }
}

impl<T: 'static, S: Storage<T>, M: WriteRefStorage> Writable for BoxedWritable<T, S, M> {
    type Mut = M;

    fn write_unchecked(&self) -> crate::WritableRef<'static, Self> {
        self.value.write_unchecked()
    }

    fn try_write_unchecked(
        &self,
    ) -> Result<crate::WritableRef<'static, Self>, generational_box::BorrowMutError> {
        self.value.try_write_unchecked()
    }
}

// We can't implement From<impl Writable<Target = T, Storage = S> + 'static> for BoxedWritable<T, S>
// because it would conflict with the From<T> for T implementation, but we can implement it for
// all specific readable types
impl<T, S: Storage<SignalData<T>>> From<Signal<T, S>>
    for BoxedWritable<T, S, SignalWriteStorage<S>>
{
    fn from(value: Signal<T, S>) -> Self {
        Self::new(value)
    }
}
impl<T, S: Storage<T>> From<CopyValue<T, S>> for BoxedWritable<T, S> {
    fn from(value: CopyValue<T, S>) -> Self {
        Self::new(value)
    }
}
impl<T: Clone + 'static, S, R: 'static, M> From<Global<T, R>> for BoxedWritable<R, S, M>
where
    T: Writable<Target = R, Storage = S, Mut = M> + InitializeFromFunction<R>,
{
    fn from(value: Global<T, R>) -> Self {
        Self::new(value)
    }
}
impl<V, O, F, FMut> From<MappedMutSignal<O, V, F, FMut>> for BoxedWritable<O, V::Storage, V::Mut>
where
    O: ?Sized,
    V: Writable + 'static,
    F: Fn(&V::Target) -> &O + 'static,
    FMut: Fn(&mut V::Target) -> &mut O + 'static,
{
    fn from(value: MappedMutSignal<O, V, F, FMut>) -> Self {
        Self::new(value)
    }
}
