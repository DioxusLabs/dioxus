use super::{Global, InitializeFromFunction};
use crate::read::ReadableExt;
use crate::read_impls;
use crate::Signal;

impl<T: 'static> InitializeFromFunction<T> for Signal<T> {
    fn initialize_from_function(f: fn() -> T) -> Self {
        Signal::new(f())
    }
}

/// A signal that can be accessed from anywhere in the application and created in a static
pub type GlobalSignal<T> = Global<Signal<T>, T>;

impl<T: 'static> GlobalSignal<T> {
    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.resolve().id()
    }

    /// Resolve the global signal. This will try to get the existing value from the current virtual dom, and if it doesn't exist, it will create a new one.
    pub fn signal(&self) -> Signal<T> {
        self.resolve()
    }
}

read_impls!(GlobalSignal<T>);
