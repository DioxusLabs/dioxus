use super::{InitializeFromFunction, LazyGlobal};
use crate::read::Readable;
use crate::read_impls;
use crate::Signal;

impl<T> InitializeFromFunction<T> for Signal<T> {
    fn initialize_from_function(f: fn() -> T) -> Self {
        Signal::new(f())
    }
}

/// A signal that can be accessed from anywhere in the application and created in a static
pub type GlobalSignal<T> = LazyGlobal<Signal<T>, T>;

impl<T: 'static> GlobalSignal<T> {
    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.resolve().id()
    }
}

read_impls!(GlobalSignal<T>);
