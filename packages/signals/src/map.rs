use crate::Signal;
use dioxus_core::ScopeId;
use std::cell::Ref;
use std::fmt::Debug;
use std::fmt::Display;

/// A signal that can only be read from.
pub struct SignalMap<T: 'static, U: ?Sized> {
    inner: Signal<T>,
    mapping: fn(&T) -> &U,
}

impl<T: 'static, U: ?Sized> SignalMap<T, U> {
    /// Create a new read-only signal.
    pub fn new(signal: Signal<T>, mapping: fn(&T) -> &U) -> Self {
        Self {
            inner: signal,
            mapping,
        }
    }

    /// Get the scope that the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
    }

    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    pub fn read(&self) -> Ref<U> {
        Ref::map(self.inner.read(), |v| (self.mapping)(v))
    }

    /// Run a closure with a reference to the signal's value.
    pub fn with<O>(&self, f: impl FnOnce(&U) -> O) -> O {
        self.inner.with(|v| f((self.mapping)(v)))
    }
}

impl<T: 'static, U: ?Sized + Clone> SignalMap<T, U> {
    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    pub fn value(&self) -> U {
        self.read().clone()
    }
}

impl<T: 'static, U: ?Sized> PartialEq for SignalMap<T, U> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T, U> std::clone::Clone for SignalMap<T, U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, U> Copy for SignalMap<T, U> {}

impl<T: 'static, U: ?Sized + Display> Display for SignalMap<T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| Display::fmt(v, f))
    }
}

impl<T: 'static, U: ?Sized + Debug> Debug for SignalMap<T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| Debug::fmt(v, f))
    }
}

impl<T: 'static, U> SignalMap<T, Vec<U>> {
    /// Read a value from the inner vector.
    pub fn get(&self, index: usize) -> Option<Ref<'_, U>> {
        Ref::filter_map(self.read(), |v| v.get(index)).ok()
    }
}

impl<T, U: Clone + 'static> SignalMap<T, Option<U>> {
    /// Unwraps the inner value and clones it.
    pub fn unwrap(&self) -> U
    where
        T: Clone,
    {
        self.with(|v| v.clone()).unwrap()
    }

    /// Attemps to read the inner value of the Option.
    pub fn as_ref(&self) -> Option<Ref<'_, U>> {
        Ref::filter_map(self.read(), |v| v.as_ref()).ok()
    }
}
