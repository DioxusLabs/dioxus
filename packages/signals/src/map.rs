use std::{ops::Deref, rc::Rc};

use crate::{read::Readable, ReadableRef};
use dioxus_core::prelude::*;
use generational_box::{AnyStorage, UnsyncStorage};

/// A read only signal that has been mapped to a new type.
pub struct MappedSignal<O: ?Sized + 'static, S: AnyStorage = UnsyncStorage> {
    try_read: Rc<dyn Fn() -> Result<S::Ref<O>, generational_box::BorrowError> + 'static>,
    peek: Rc<dyn Fn() -> S::Ref<O> + 'static>,
}

impl<O: ?Sized, S: AnyStorage> Clone for MappedSignal<O, S> {
    fn clone(&self) -> Self {
        MappedSignal {
            try_read: self.try_read.clone(),
            peek: self.peek.clone(),
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
        try_read: Rc<dyn Fn() -> Result<S::Ref<O>, generational_box::BorrowError> + 'static>,
        peek: Rc<dyn Fn() -> S::Ref<O> + 'static>,
    ) -> Self {
        MappedSignal { try_read, peek }
    }
}

impl<O, S> Readable for MappedSignal<O, S>
where
    O: ?Sized,
    S: AnyStorage,
{
    type Target = O;
    type Storage = S;

    fn try_read(&self) -> Result<ReadableRef<Self>, generational_box::BorrowError> {
        (self.try_read)()
    }

    fn peek(&self) -> ReadableRef<Self> {
        (self.peek)()
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
        std::ptr::eq(&self.peek, &other.peek) && std::ptr::eq(&self.try_read, &other.try_read)
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
        Readable::deref_impl(self)
    }
}
