use std::{ops::Deref, rc::Rc};

use crate::read::Readable;
use dioxus_core::prelude::*;

/// A read only signal that has been mapped to a new type.
pub struct MappedSignal<O: ?Sized + 'static, R: Readable> {
    readable: R,
    mapping: Rc<dyn Fn(&R::Target) -> &O + 'static>,
}

impl<O: ?Sized, R: Readable + Clone> Clone for MappedSignal<O, R> {
    fn clone(&self) -> Self {
        MappedSignal {
            readable: self.readable.clone(),
            mapping: self.mapping.clone(),
        }
    }
}

impl<O, R> MappedSignal<O, R>
where
    O: ?Sized,
    R: Readable + 'static,
{
    /// Create a new mapped signal.
    pub(crate) fn new(readable: R, mapping: impl Fn(&R::Target) -> &O + 'static) -> Self {
        MappedSignal {
            readable,
            mapping: Rc::new(mapping),
        }
    }
}

impl<O, R> Readable for MappedSignal<O, R>
where
    O: ?Sized,
    R: Readable,
{
    type Target = O;
    type Ref<J: ?Sized + 'static> = R::Ref<J>;

    fn map_ref<I: ?Sized, U: ?Sized, F: FnOnce(&I) -> &U>(
        ref_: Self::Ref<I>,
        f: F,
    ) -> Self::Ref<U> {
        R::map_ref(ref_, f)
    }

    fn try_map_ref<I: ?Sized, U: ?Sized, F: FnOnce(&I) -> Option<&U>>(
        ref_: Self::Ref<I>,
        f: F,
    ) -> Option<Self::Ref<U>> {
        R::try_map_ref(ref_, f)
    }

    fn try_read(&self) -> Result<Self::Ref<O>, generational_box::BorrowError> {
        self.readable
            .try_read()
            .map(|ref_| R::map_ref(ref_, |r| (self.mapping)(r)))
    }

    fn peek(&self) -> Self::Ref<Self::Target> {
        R::map_ref(self.readable.peek(), |r| (self.mapping)(r))
    }
}

impl<O, R> IntoAttributeValue for MappedSignal<O, R>
where
    O: Clone + IntoAttributeValue + ?Sized,
    R: Readable + 'static,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<O, R> PartialEq for MappedSignal<O, R>
where
    O: ?Sized,
    R: PartialEq + Readable + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.readable == other.readable && std::ptr::eq(&self.mapping, &other.mapping)
    }
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<O, R> Deref for MappedSignal<O, R>
where
    O: Clone,
    R: Readable + 'static,
{
    type Target = dyn Fn() -> O;

    fn deref(&self) -> &Self::Target {
        Readable::deref_impl(self)
    }
}
