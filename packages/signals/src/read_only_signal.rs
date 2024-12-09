use crate::{read::Readable, ReadableRef, Signal, SignalData};
use dioxus_core::IntoDynNode;
use std::ops::Deref;

use crate::{default_impl, read_impls};
use dioxus_core::{prelude::IntoAttributeValue, ScopeId};
use generational_box::{BorrowResult, Storage, UnsyncStorage};

/// A signal that can only be read from.
pub struct ReadOnlySignal<T: 'static, S: Storage<SignalData<T>> = UnsyncStorage> {
    inner: Signal<T, S>,
}

/// A signal that can only be read from.
pub type ReadSignal<T, S> = ReadOnlySignal<T, S>;

impl<T: 'static, S: Storage<SignalData<T>>> From<Signal<T, S>> for ReadOnlySignal<T, S> {
    fn from(inner: Signal<T, S>) -> Self {
        Self { inner }
    }
}

impl<T: 'static> ReadOnlySignal<T> {
    /// Create a new read-only signal.
    #[track_caller]
    pub fn new(signal: Signal<T>) -> Self {
        Self::new_maybe_sync(signal)
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> ReadOnlySignal<T, S> {
    /// Create a new read-only signal that is maybe sync.
    #[track_caller]
    pub fn new_maybe_sync(signal: Signal<T, S>) -> Self {
        Self { inner: signal }
    }

    /// Get the scope that the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
    }

    /// Get the id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.inner.id()
    }

    /// Point to another signal
    pub fn point_to(&self, other: Self) -> BorrowResult {
        self.inner.point_to(other.inner)
    }

    #[doc(hidden)]
    /// This is only used by the `props` macro.
    /// Mark any readers of the signal as dirty
    pub fn mark_dirty(&mut self) {
        use crate::write::Writable;
        use warnings::Warning;
        // We diff props while rendering, but we only write to the signal if it has
        // changed so it is safe to ignore the warning
        crate::warnings::signal_write_in_component_body::allow(|| {
            _ = self.inner.try_write();
        });
    }
}

impl<T, S: Storage<SignalData<T>>> Readable for ReadOnlySignal<T, S> {
    type Target = T;
    type Storage = S;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.inner.try_read_unchecked()
    }

    /// Get the current value of the signal. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<S::Ref<'static, T>> {
        self.inner.try_peek_unchecked()
    }
}

#[cfg(feature = "serialize")]
impl<T: serde::Serialize + 'static, Store: Storage<SignalData<T>>> serde::Serialize
    for ReadOnlySignal<T, Store>
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.read().serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de, T: serde::Deserialize<'de> + 'static, Store: Storage<SignalData<T>>>
    serde::Deserialize<'de> for ReadOnlySignal<T, Store>
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::new_maybe_sync(Signal::new_maybe_sync(
            T::deserialize(deserializer)?,
        )))
    }
}

impl<T> IntoAttributeValue for ReadOnlySignal<T>
where
    T: Clone + IntoAttributeValue,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T> IntoDynNode for ReadOnlySignal<T>
where
    T: Clone + IntoDynNode,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self().into_dyn_node()
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> PartialEq for ReadOnlySignal<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: Clone, S: Storage<SignalData<T>> + 'static> Deref for ReadOnlySignal<T, S> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { Readable::deref_impl(self) }
    }
}

read_impls!(
    ReadOnlySignal<T, S> where
        S: Storage<SignalData<T>>
);
default_impl!(
    ReadOnlySignal<T, S> where
    S: Storage<SignalData<T>>
);

impl<T: 'static, S: Storage<SignalData<T>>> Clone for ReadOnlySignal<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> Copy for ReadOnlySignal<T, S> {}
