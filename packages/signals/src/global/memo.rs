use crate::read_impls;
use crate::{read::Readable, Memo, ReadableRef};
use generational_box::{BorrowResult, UnsyncStorage};
use std::ops::Deref;

use super::{InitializeFromFunction, LazyGlobal};

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

impl<T: PartialEq + 'static> Readable for GlobalMemo<T> {
    type Target = T;
    type Storage = UnsyncStorage;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.resolve().try_read_unchecked()
    }

    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>> {
        self.resolve().try_peek_unchecked()
    }
}

/// Allow calling a signal with memo() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<T: PartialEq + Clone + 'static> Deref for GlobalMemo<T> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        Readable::deref_impl(self)
    }
}

read_impls!(GlobalMemo<T> where T: PartialEq);
