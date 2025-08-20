use super::{Global, InitializeFromFunction};
use crate::read::ReadableExt;
use crate::read_impls;
use crate::Memo;

impl<T: PartialEq + 'static> InitializeFromFunction<T> for Memo<T> {
    fn initialize_from_function(f: fn() -> T) -> Self {
        Memo::new(f)
    }
}

/// A memo that can be accessed from anywhere in the application and created in a static
pub type GlobalMemo<T> = Global<Memo<T>, T>;

impl<T: PartialEq + 'static> GlobalMemo<T> {
    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.resolve().id()
    }

    /// Resolve the global memo. This will try to get the existing value from the current virtual dom, and if it doesn't exist, it will create a new one.
    pub fn memo(&self) -> Memo<T> {
        self.resolve()
    }
}

read_impls!(GlobalMemo<T> where T: PartialEq);
