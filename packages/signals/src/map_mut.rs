use std::ops::Deref;

use crate::{
    read_impls, write_impls, Readable, ReadableExt, ReadableRef, Writable, WritableExt,
    WritableRef, WriteLock,
};
use dioxus_core::{IntoAttributeValue, Subscribers};
use generational_box::{AnyStorage, BorrowResult};

/// A read only signal that has been mapped to a new type.
pub struct MappedMutSignal<
    O: ?Sized,
    V,
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
    V: Clone,
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

impl<V, O, F, FMut> Copy for MappedMutSignal<O, V, F, FMut>
where
    V: Copy,
    F: Copy,
    FMut: Copy,
{
}

impl<V, O, F, FMut> MappedMutSignal<O, V, F, FMut>
where
    O: ?Sized,
{
    /// Create a new mapped signal.
    pub fn new(value: V, map_fn: F, map_fn_mut: FMut) -> Self {
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
    V::Target: 'static,
    F: Fn(&V::Target) -> &O,
{
    type Target = O;
    type Storage = V::Storage;

    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>
    where
        O: 'static,
    {
        let value = self.value.try_read_unchecked()?;
        Ok(V::Storage::map(value, |v| (self.map_fn)(v)))
    }

    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>>
    where
        O: 'static,
    {
        let value = self.value.try_peek_unchecked()?;
        Ok(V::Storage::map(value, |v| (self.map_fn)(v)))
    }

    fn subscribers(&self) -> Subscribers
    where
        O: 'static,
    {
        self.value.subscribers()
    }
}

impl<V, O, F, FMut> Writable for MappedMutSignal<O, V, F, FMut>
where
    O: ?Sized,
    V: Writable,
    V::Target: 'static,
    F: Fn(&V::Target) -> &O,
    FMut: Fn(&mut V::Target) -> &mut O,
{
    type WriteMetadata = V::WriteMetadata;

    fn try_write_unchecked(
        &self,
    ) -> Result<WritableRef<'static, Self>, generational_box::BorrowMutError> {
        let value = self.value.try_write_unchecked()?;
        Ok(WriteLock::map(value, |v| (self.map_fn_mut)(v)))
    }
}

impl<V, O, F, FMut> IntoAttributeValue for MappedMutSignal<O, V, F, FMut>
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
    O: Clone + 'static,
    V: Readable + 'static,
    V::Target: 'static,
    F: Fn(&V::Target) -> &O + 'static,
    FMut: 'static,
{
    type Target = dyn Fn() -> O;

    fn deref(&self) -> &Self::Target {
        crate::readable_deref_impl(self)
    }
}

read_impls!(MappedMutSignal<T, V, F, FMut> where V: Readable<Target: 'static>, F: Fn(&V::Target) -> &T);
write_impls!(MappedMutSignal<T, V, F, FMut> where V: Writable<Target: 'static>, F: Fn(&V::Target) -> &T, FMut: Fn(&mut V::Target) -> &mut T);
