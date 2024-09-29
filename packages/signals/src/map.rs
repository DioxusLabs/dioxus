use std::{ops::Deref, rc::Rc};

use crate::{read::Readable, read_impls, ReadableRef};
use dioxus_core::prelude::*;
use generational_box::{AnyStorage, BorrowResult, UnsyncStorage};

/// A read only signal that has been mapped to a new type.
pub struct MappedSignal<O: ?Sized + 'static, S: AnyStorage = UnsyncStorage> {
    try_read: Rc<dyn Fn() -> Result<S::Ref<'static, O>, generational_box::BorrowError> + 'static>,
    try_peek: Rc<dyn Fn() -> Result<S::Ref<'static, O>, generational_box::BorrowError> + 'static>,
}

impl<O: ?Sized, S: AnyStorage> Clone for MappedSignal<O, S> {
    fn clone(&self) -> Self {
        MappedSignal {
            try_read: self.try_read.clone(),
            try_peek: self.try_peek.clone(),
        }
    }
}

impl<O, S> MappedSignal<O, S>
where
    O: ?Sized,
    S: AnyStorage,
{
    /// Create a new mapped signal.
    pub(crate) fn new(
        try_read: Rc<
            dyn Fn() -> Result<S::Ref<'static, O>, generational_box::BorrowError> + 'static,
        >,
        try_peek: Rc<
            dyn Fn() -> Result<S::Ref<'static, O>, generational_box::BorrowError> + 'static,
        >,
    ) -> Self {
        MappedSignal { try_read, try_peek }
    }
}

impl<O, S> Readable for MappedSignal<O, S>
where
    O: ?Sized,
    S: AnyStorage,
{
    type Target = O;
    type Storage = S;

    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        (self.try_read)()
    }

    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>> {
        (self.try_peek)()
    }
}

impl<O, S> IntoAttributeValue for MappedSignal<O, S>
where
    O: Clone + IntoAttributeValue,
    S: AnyStorage,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<O, S> PartialEq for MappedSignal<O, S>
where
    O: ?Sized,
    S: AnyStorage,
{
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.try_peek, &other.try_peek)
            && std::ptr::eq(&self.try_read, &other.try_read)
    }
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<O, S> Deref for MappedSignal<O, S>
where
    O: Clone,
    S: AnyStorage + 'static,
{
    type Target = dyn Fn() -> O;

    fn deref(&self) -> &Self::Target {
        unsafe { Readable::deref_impl(self) }
    }
}

read_impls!(MappedSignal<T, S: AnyStorage>);
