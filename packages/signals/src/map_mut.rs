use std::ops::Deref;

use crate::{read_impls, write_impls, Readable, ReadableRef, Writable, WritableRef};
use dioxus_core::prelude::*;
use generational_box::{AnyStorage, BorrowResult};

/// A read only signal that has been mapped to a new type.
pub struct MappedMutSignal<
    O: ?Sized + 'static,
    V: Readable,
    F = fn(&<V as Readable>::Target) -> &O,
    FMut = fn(&mut <V as Readable>::Target) -> &mut O,
> {
    value: V,
    map_fn: F,
    map_fn_mut: FMut,
    _marker: std::marker::PhantomData<O>,
}

impl<V, O, F, FMut> Clone for MappedMutSignal<O, V, F, FMut>
where
    V: Readable + Clone,
    F: Clone,
    FMut: Clone,
{
    fn clone(&self) -> Self {
        MappedMutSignal {
            value: self.value.clone(),
            map_fn: self.map_fn.clone(),
            map_fn_mut: self.map_fn_mut.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<V, O, F, FMut> MappedMutSignal<O, V, F, FMut>
where
    O: ?Sized,
    V: Readable,
    F: Fn(&V::Target) -> &O,
{
    /// Create a new mapped signal.
    pub(crate) fn new(value: V, map_fn: F, map_fn_mut: FMut) -> Self {
        MappedMutSignal {
            value,
            map_fn,
            map_fn_mut,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<V, O, F, FMut> Readable for MappedMutSignal<O, V, F, FMut>
where
    O: ?Sized,
    V: Readable,
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
}

impl<V, O, F, FMut> Writable for MappedMutSignal<O, V, F, FMut>
where
    O: ?Sized,
    V: Writable,
    F: Fn(&V::Target) -> &O,
    FMut: Fn(&mut V::Target) -> &mut O,
{
    type Mut<'a, R: ?Sized + 'static> = WritableRef<'a, V, R>;

    fn map_ref_mut<I: ?Sized, U: ?Sized, F_: FnOnce(&mut I) -> &mut U>(
        ref_: Self::Mut<'_, I>,
        f: F_,
    ) -> Self::Mut<'_, U> {
        V::map_ref_mut(ref_, f)
    }

    fn try_map_ref_mut<I: ?Sized, U: ?Sized, F_: FnOnce(&mut I) -> Option<&mut U>>(
        ref_: Self::Mut<'_, I>,
        f: F_,
    ) -> Option<Self::Mut<'_, U>> {
        V::try_map_ref_mut(ref_, f)
    }

    fn downcast_lifetime_mut<'a: 'b, 'b, T: ?Sized + 'static>(
        mut_: Self::Mut<'a, T>,
    ) -> Self::Mut<'b, T> {
        V::downcast_lifetime_mut(mut_)
    }

    fn try_write_unchecked(
        &self,
    ) -> Result<WritableRef<'static, Self>, generational_box::BorrowMutError> {
        let value = self.value.try_write_unchecked()?;
        Ok(V::map_ref_mut(value, |v| (self.map_fn_mut)(v)))
    }
}

impl<V, O, F, FMut> IntoAttributeValue for MappedMutSignal<O, V, F, FMut>
where
    O: ?Sized + Clone + IntoAttributeValue,
    V: Readable,
    F: Fn(&V::Target) -> &O,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<V, O, F, FMut> PartialEq for MappedMutSignal<O, V, F, FMut>
where
    O: ?Sized,
    V: Readable + PartialEq,
    F: PartialEq,
    FMut: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.map_fn == other.map_fn
            && self.map_fn_mut == other.map_fn_mut
    }
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to clone types, though could probably specialize for string/arc/rc
impl<V, O, F, FMut> Deref for MappedMutSignal<O, V, F, FMut>
where
    O: Clone + ?Sized,
    V: Readable + 'static,
    F: Fn(&V::Target) -> &O + 'static,
    FMut: 'static,
{
    type Target = dyn Fn() -> O;

    fn deref(&self) -> &Self::Target {
        unsafe { Readable::deref_impl(self) }
    }
}

read_impls!(MappedMutSignal<T, V, F, FMut> where V: Readable, F: Fn(&V::Target) -> &T);
write_impls!(MappedMutSignal<T, V, F, FMut> where V: Writable, F: Fn(&V::Target) -> &T, FMut: Fn(&mut V::Target) -> &mut T);
