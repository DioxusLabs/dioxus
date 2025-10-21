use crate::{read::Readable, read_impls, ReadableExt, ReadableRef};
use dioxus_core::{IntoAttributeValue, Subscribers};
use generational_box::{AnyStorage, BorrowResult};
use std::ops::Deref;

/// A read only signal that has been mapped to a new type.
pub struct MappedSignal<O: ?Sized, V, F = fn(&<V as Readable>::Target) -> &O> {
    value: V,
    map_fn: F,
    _marker: std::marker::PhantomData<O>,
}

impl<V, O, F> Clone for MappedSignal<O, V, F>
where
    V: Readable + Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        MappedSignal {
            value: self.value.clone(),
            map_fn: self.map_fn.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<V, O, F> Copy for MappedSignal<O, V, F>
where
    V: Readable + Copy,
    F: Copy,
{
}

impl<V, O, F> MappedSignal<O, V, F>
where
    O: ?Sized,
{
    /// Create a new mapped signal.
    pub fn new(value: V, map_fn: F) -> Self {
        MappedSignal {
            value,
            map_fn,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<V, O, F> Readable for MappedSignal<O, V, F>
where
    O: ?Sized,
    V: Readable,
    V::Target: 'static,
    F: Fn(&V::Target) -> &O,
{
    type Target = O;
    type Storage = V::Storage;

    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        let value = self.value.try_read_unchecked()?;
        Ok(V::Storage::map(value, |v| (self.map_fn)(v)))
    }

    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>> {
        let value = self.value.try_peek_unchecked()?;
        Ok(V::Storage::map(value, |v| (self.map_fn)(v)))
    }

    fn subscribers(&self) -> Subscribers {
        self.value.subscribers()
    }
}

impl<V, O, F> IntoAttributeValue for MappedSignal<O, V, F>
where
    O: Clone + IntoAttributeValue + 'static,
    V: Readable,
    V::Target: 'static,
    F: Fn(&V::Target) -> &O,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<V, O, F> PartialEq for MappedSignal<O, V, F>
where
    O: ?Sized,
    V: Readable + PartialEq,
    F: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.map_fn == other.map_fn
    }
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to clone types, though could probably specialize for string/arc/rc
impl<V, O, F> Deref for MappedSignal<O, V, F>
where
    O: Clone + 'static,
    V: Readable + 'static,
    F: Fn(&V::Target) -> &O + 'static,
{
    type Target = dyn Fn() -> O;

    fn deref(&self) -> &Self::Target {
        crate::readable_deref_impl(self)
    }
}

read_impls!(MappedSignal<T, V, F> where V: Readable<Target: 'static>, F: Fn(&V::Target) -> &T);
