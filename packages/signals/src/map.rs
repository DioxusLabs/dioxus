use crate::Signal;
use dioxus_core::ScopeId;
use std::cell::Ref;
use std::fmt::Debug;
use std::fmt::Display;

/// A read only signal that has been mapped to a new type.
pub struct SignalMap<U: ?Sized> {
    mapping: Box<dyn Fn() -> Ref<'static, U>>,
}

impl<T: 'static, U: ?Sized> SignalMap<T, U> {
    /// Create a new mapped signal.
    pub fn new(signal: Signal<T>, mapping: fn(&T) -> &U) -> Self {
        Self {
            mapping: Box::new(move || Ref::map(signal.read(), |v| (mapping)(v))),
        }
    }

    /// Get the scope that the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
    }

    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    pub fn read(&self) -> Ref<U> {
        (self.mapping)()
    }

    /// Run a closure with a reference to the signal's value.
    pub fn with<O>(&self, f: impl FnOnce(&U) -> O) -> O {
        f(&*self.read())
    }
}

impl<U: ?Sized + Clone> SignalMap<U> {
    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    pub fn value(&self) -> U {
        self.read().clone()
    }
}

impl<U: ?Sized> PartialEq for SignalMap<U> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<U> std::clone::Clone for SignalMap<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> Copy for SignalMap<U> {}

impl<U: ?Sized + Display> Display for SignalMap<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| Display::fmt(v, f))
    }
}

impl<U: ?Sized + Debug> Debug for SignalMap<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| Debug::fmt(v, f))
    }
}

impl<U> SignalMap<Vec<U>> {
    /// Read a value from the inner vector.
    pub fn get(&self, index: usize) -> Option<Ref<'static, U>> {
        Ref::filter_map(self.read(), |v| v.get(index)).ok()
    }
}

impl<U: Clone + 'static> SignalMap<Option<U>> {
    /// Unwraps the inner value and clones it.
    pub fn unwrap(&self) -> U
    where
        T: Clone,
    {
        self.with(|v| v.clone()).unwrap()
    }

    /// Attemps to read the inner value of the Option.
    pub fn as_ref(&self) -> Option<Ref<'static, U>> {
        Ref::filter_map(self.read(), |v| v.as_ref()).ok()
    }
}
