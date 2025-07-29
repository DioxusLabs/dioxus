use crate::SelectorScope;
use dioxus_signals::{MappedMutSignal, UnsyncStorage, WriteSignal};
use std::marker::PhantomData;

pub struct Store<T: ?Sized, W> {
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

    /// Get the underlying selector.
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
    W: crate::macro_helpers::dioxus_signals::Writable<Storage = UnsyncStorage> + 'static,
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
impl<T: ?Sized, W> crate::macro_helpers::dioxus_signals::Readable for Store<T, W>
where
    W: crate::macro_helpers::dioxus_signals::Readable<Target = T> + 'static,
    T: 'static,
{
    type Storage = W::Storage;
    type Target = T;
    fn try_read_unchecked(
        &self,
    ) -> Result<
        crate::macro_helpers::dioxus_signals::ReadableRef<'static, Self>,
        crate::macro_helpers::dioxus_signals::BorrowError,
    > {
        self.selector.try_read_unchecked()
    }
    fn try_peek_unchecked(
        &self,
    ) -> Result<
        crate::macro_helpers::dioxus_signals::ReadableRef<'static, Self>,
        crate::macro_helpers::dioxus_signals::BorrowError,
    > {
        self.selector.try_peek_unchecked()
    }
    fn subscribers(&self) -> Option<crate::macro_helpers::dioxus_core::Subscribers> {
        self.selector.subscribers()
    }
}
impl<T: ?Sized, W> crate::macro_helpers::dioxus_signals::Writable for Store<T, W>
where
    W: crate::macro_helpers::dioxus_signals::Writable<Target = T> + 'static,
    T: 'static,
{
    type WriteMetadata = W::WriteMetadata;
    fn try_write_unchecked(
        &self,
    ) -> Result<
        crate::macro_helpers::dioxus_signals::WritableRef<'static, Self>,
        crate::macro_helpers::dioxus_signals::BorrowMutError,
    > {
        self.selector.try_write_unchecked()
    }
}

impl<T: ?Sized, W> ::std::fmt::Debug for Store<T, W>
where
    Self: crate::macro_helpers::dioxus_signals::Readable<Target = T>,
    T: ::std::fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        crate::macro_helpers::dioxus_signals::ReadableExt::read(self).fmt(f)
    }
}
impl<T: ?Sized, W> ::std::fmt::Display for Store<T, W>
where
    Self: crate::macro_helpers::dioxus_signals::Readable<Target = T>,
    T: ::std::fmt::Display + 'static,
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        crate::macro_helpers::dioxus_signals::ReadableExt::read(self).fmt(f)
    }
}
impl<T: ?Sized, W> crate::macro_helpers::dioxus_core::IntoAttributeValue for Store<T, W>
where
    Self: crate::macro_helpers::dioxus_signals::Readable<Target = T>,
    T: ::std::clone::Clone + crate::macro_helpers::dioxus_core::IntoAttributeValue + 'static,
{
    fn into_value(self) -> crate::macro_helpers::dioxus_core::AttributeValue {
        crate::macro_helpers::dioxus_signals::ReadableExt::cloned(&self).into_value()
    }
}
impl<T: ?Sized, W> crate::macro_helpers::dioxus_core::IntoDynNode for Store<T, W>
where
    Self: crate::macro_helpers::dioxus_signals::Readable<Target = T>,
    T: ::std::clone::Clone + crate::macro_helpers::dioxus_core::IntoDynNode + 'static,
{
    fn into_dyn_node(self) -> crate::macro_helpers::dioxus_core::DynamicNode {
        crate::macro_helpers::dioxus_signals::ReadableExt::cloned(&self).into_dyn_node()
    }
}
impl<T: ?Sized, W> ::std::ops::Deref for Store<T, W>
where
    Self: crate::macro_helpers::dioxus_signals::Readable<Target = T> + 'static,
    T: ::std::clone::Clone + 'static,
{
    type Target = dyn Fn() -> T;
    fn deref(&self) -> &Self::Target {
        unsafe { crate::macro_helpers::dioxus_signals::ReadableExt::deref_impl(self) }
    }
}
