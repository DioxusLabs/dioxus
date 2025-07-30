use crate::SelectorScope;
use dioxus_core::{AttributeValue, DynamicNode, IntoAttributeValue, IntoDynNode, Subscribers};
use dioxus_signals::{
    read_impls, write_impls, BorrowError, BorrowMutError, MappedMutSignal, Readable, ReadableExt,
    ReadableRef, UnsyncStorage, Writable, WritableExt, WritableRef, WriteSignal,
};
use std::marker::PhantomData;

pub struct Store<T: ?Sized, W = WriteSignal<T>> {
    selector: SelectorScope<W>,
    _phantom: PhantomData<Box<T>>,
}

impl<T: ?Sized, W> Store<T, W> {
    /// Creates a new `Store` with the given selector.
    pub fn new(selector: SelectorScope<W>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }

    /// Get the underlying selector. You should generally not use this directly. Instead use the extension
    /// traits implemented for your `Store` type.
    pub fn selector(&self) -> &SelectorScope<W> {
        &self.selector
    }
}

impl<T: ?Sized, W> PartialEq for Store<T, W>
where
    W: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.selector == other.selector
    }
}
impl<T: ?Sized, W> Clone for Store<T, W>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            _phantom: ::std::marker::PhantomData,
        }
    }
}
impl<T: ?Sized, W> Copy for Store<T, W> where W: Copy {}

impl<__F, __FMut, T: ?Sized, W> ::std::convert::From<Store<T, MappedMutSignal<T, W, __F, __FMut>>>
    for Store<T, WriteSignal<T>>
where
    W: Writable<Storage = UnsyncStorage> + 'static,
    __F: Fn(&W::Target) -> &T + 'static,
    __FMut: Fn(&mut W::Target) -> &mut T + 'static,
{
    fn from(value: Store<T, MappedMutSignal<T, W, __F, __FMut>>) -> Self {
        Store {
            selector: value.selector.map(::std::convert::Into::into),
            _phantom: ::std::marker::PhantomData,
        }
    }
}
impl<T: ?Sized, W> Readable for Store<T, W>
where
    W: Readable<Target = T>,
    T: 'static,
{
    type Storage = W::Storage;
    type Target = T;
    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError> {
        self.selector.try_read_unchecked()
    }
    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError> {
        self.selector.try_peek_unchecked()
    }
    fn subscribers(&self) -> Option<Subscribers> {
        self.selector.subscribers()
    }
}
impl<T: ?Sized, W> Writable for Store<T, W>
where
    W: Writable<Target = T>,
    T: 'static,
{
    type WriteMetadata = W::WriteMetadata;
    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError> {
        self.selector.try_write_unchecked()
    }
}
impl<T: ?Sized, W> IntoAttributeValue for Store<T, W>
where
    Self: Readable<Target = T>,
    T: ::std::clone::Clone + IntoAttributeValue + 'static,
{
    fn into_value(self) -> AttributeValue {
        ReadableExt::cloned(&self).into_value()
    }
}
impl<T: ?Sized, W> IntoDynNode for Store<T, W>
where
    Self: Readable<Target = T>,
    T: ::std::clone::Clone + IntoDynNode + 'static,
{
    fn into_dyn_node(self) -> DynamicNode {
        ReadableExt::cloned(&self).into_dyn_node()
    }
}
impl<T: ?Sized, W> ::std::ops::Deref for Store<T, W>
where
    Self: Readable<Target = T> + 'static,
    T: ::std::clone::Clone + 'static,
{
    type Target = dyn Fn() -> T;
    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

read_impls!(Store<T, W> where W: Readable<Target = T>);
write_impls!(Store<T, W> where W: Writable<Target = T>);
