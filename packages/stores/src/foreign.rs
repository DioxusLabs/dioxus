use crate::{CreateSelector, SelectorScope, Storable};
use dioxus_core::{IntoAttributeValue, IntoDynNode, Subscribers};
use dioxus_signals::{
    read_impls, write_impls, BorrowError, BorrowMutError, Readable, ReadableExt, ReadableRef,
    Writable, WritableExt, WritableRef,
};
use std::{marker::PhantomData, ops::Deref};

pub struct ForeignType<T> {
    phantom: PhantomData<(T,)>,
}

impl<T> Storable for ForeignType<T> {
    type Store<View> = ForeignStore<T, View>;
}

pub struct ForeignStore<T, W> {
    selector: SelectorScope<W>,
    phantom: PhantomData<T>,
}

impl<W, T> Clone for ForeignStore<T, W>
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

impl<W, T> Copy for ForeignStore<T, W> where W: Copy {}

impl<W, T> CreateSelector for ForeignStore<T, W> {
    type View = W;

    fn new(selector: SelectorScope<Self::View>) -> Self {
        Self {
            selector,
            phantom: PhantomData,
        }
    }
}

read_impls!(ForeignStore<T, W> where W: Readable<Target = T>);
write_impls!(ForeignStore<T, W> where W: Writable<Target = T>);

impl<T, W> IntoAttributeValue for ForeignStore<T, W>
where
    T: Clone + IntoAttributeValue + 'static,
    W: Writable<Target = T>,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T, W> IntoDynNode for ForeignStore<T, W>
where
    T: Clone + IntoDynNode + 'static,
    W: Writable<Target = T>,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self.with(|f| f.clone().into_dyn_node())
    }
}

impl<T: Clone + 'static, W: Writable<Target = T> + 'static> Deref for ForeignStore<T, W> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<W, T: 'static> Readable for ForeignStore<T, W>
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

impl<W, T: 'static> Writable for ForeignStore<T, W>
where
    W: Writable<Target = T>,
{
    type WriteMetadata = <W as Writable>::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError> {
        self.selector.try_write_unchecked()
    }
}
