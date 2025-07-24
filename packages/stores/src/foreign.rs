use crate::{CreateSelector, SelectorScope, SelectorStorage, Storable};
use dioxus_core::{IntoAttributeValue, IntoDynNode};
use dioxus_signals::{
    read_impls, write_impls, BorrowError, BorrowMutError, Readable, ReadableExt, ReadableRef,
    Subscribers, UnsyncStorage, Writable, WritableExt, WritableRef,
};
use std::{marker::PhantomData, ops::Deref};

pub struct ForeignType<T, S: SelectorStorage = UnsyncStorage> {
    phantom: PhantomData<(T, S)>,
}

impl<T, S: SelectorStorage> Storable for ForeignType<T, S> {
    type Store<View, St: SelectorStorage> = ForeignStore<T, View, St>;
}

pub struct ForeignStore<T, W, S: SelectorStorage = UnsyncStorage> {
    selector: SelectorScope<W, S>,
    phantom: PhantomData<T>,
}

impl<W, T, S: SelectorStorage> Clone for ForeignStore<T, W, S>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            phantom: PhantomData,
        }
    }
}

impl<W, T, S: SelectorStorage> Copy for ForeignStore<T, W, S> where W: Copy {}

impl<W, T, S: SelectorStorage> CreateSelector for ForeignStore<T, W, S> {
    type View = W;
    type Storage = S;

    fn new(selector: SelectorScope<Self::View, Self::Storage>) -> Self {
        Self {
            selector,
            phantom: PhantomData,
        }
    }
}

read_impls!(ForeignStore<T, W, S> where W: Readable<Target = T>, S: SelectorStorage);
write_impls!(ForeignStore<T, W, S> where W: Writable<Target = T>, S: SelectorStorage);

impl<T, W, S> IntoAttributeValue for ForeignStore<T, W, S>
where
    T: Clone + IntoAttributeValue + 'static,
    W: Writable<Target = T>,
    S: SelectorStorage,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T, W, S> IntoDynNode for ForeignStore<T, W, S>
where
    T: Clone + IntoDynNode + 'static,
    W: Writable<Target = T>,
    S: SelectorStorage,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self.with(|f| f.clone().into_dyn_node())
    }
}

impl<T: Clone + 'static, W: Writable<Target = T> + 'static, S: SelectorStorage> Deref
    for ForeignStore<T, W, S>
{
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<W, T: 'static, S: SelectorStorage> Readable for ForeignStore<T, W, S>
where
    W: Readable<Target = T>,
{
    type Target = T;

    type Storage = W::Storage;

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

impl<W, T: 'static, S: SelectorStorage> Writable for ForeignStore<T, W, S>
where
    W: Writable<Target = T>,
{
    type WriteMetadata = <W as Writable>::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError> {
        self.selector.try_write_unchecked()
    }
}
