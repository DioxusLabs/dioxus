use crate::{read::Readable, Signal, SignalData};
use std::ops::Deref;

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
        self.inner.take()
    }
}

impl<T, S: Storage<SignalData<T>>> Readable<T> for ReadOnlySignal<T, S> {
    type Ref<R: ?Sized + 'static> = S::Ref<R>;

    fn map_ref<I, U: ?Sized, F: FnOnce(&I) -> &U>(ref_: Self::Ref<I>, f: F) -> Self::Ref<U> {
        S::map(ref_, f)
    }

    fn try_map_ref<I, U: ?Sized, F: FnOnce(&I) -> Option<&U>>(
        ref_: Self::Ref<I>,
        f: F,
    ) -> Option<Self::Ref<U>> {
        S::try_map(ref_, f)
    }

    /// Get the current value of the signal. This will subscribe the current scope to the signal.  If you would like to read the signal without subscribing to it, you can use [`Self::peek`] instead.
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    fn read(&self) -> S::Ref<T> {
        self.inner.read()
    }

    /// Get the current value of the signal. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    fn peek(&self) -> S::Ref<T> {
        self.inner.peek()
    }
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize + 'static, Store: Storage<SignalData<T>>> serde::Serialize
    for ReadOnlySignal<T, Store>
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.read().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
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
