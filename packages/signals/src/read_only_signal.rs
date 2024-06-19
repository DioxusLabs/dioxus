use crate::{read::Readable, ReadableRef, Signal, SignalData};
use dioxus_core::IntoDynNode;
use std::ops::Deref;

use crate::{default_impl, read_impls};
use dioxus_core::{prelude::IntoAttributeValue, ScopeId};
use generational_box::{Storage, UnsyncStorage};

/// A signal that can only be read from.
pub struct ReadOnlySignal<T: 'static, S: Storage<SignalData<T>> = UnsyncStorage> {
    inner: Signal<T, S>,
}

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

    #[doc(hidden)]
    /// This should only be used by the `rsx!` macro.
    pub fn __set(&mut self, value: T) {
        use crate::write::Writable;
        self.inner.set(value);
    }

    #[doc(hidden)]
    /// This should only be used by the `rsx!` macro.
    pub fn __take(&self) -> T {
        self.inner
            .manually_drop()
            .expect("Signal has already been dropped")
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
    fn peek_unchecked(&self) -> S::Ref<'static, T> {
        self.inner.peek_unchecked()
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
        Readable::deref_impl(self)
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
