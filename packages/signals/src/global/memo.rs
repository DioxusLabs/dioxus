use super::{InitializeFromFunction, LazyGlobal};
use crate::read::Readable;
use crate::read_impls;
use crate::Memo;

impl<T: PartialEq> InitializeFromFunction<T> for Memo<T> {
    fn initialize_from_function(f: fn() -> T) -> Self {
        Memo::new(f)
    }
}

/// A memo that can be accessed from anywhere in the application and created in a static
pub type GlobalMemo<T> = LazyGlobal<Memo<T>, T>;

impl<T: PartialEq + 'static> GlobalMemo<T> {
    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.resolve().id()
    }
}

read_impls!(GlobalMemo<T> where T: PartialEq);
