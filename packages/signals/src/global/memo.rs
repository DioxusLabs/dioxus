use crate::{read::Readable, Memo, ReadableRef};
use crate::{read_impls, GlobalKey};
use dioxus_core::prelude::ScopeId;
use generational_box::UnsyncStorage;
use std::ops::Deref;

use crate::Signal;

use super::get_global_context;

/// A signal that can be accessed from anywhere in the application and created in a static
pub struct GlobalMemo<T: 'static> {
    selector: fn() -> T,
    key: GlobalKey<'static>,
}

impl<T: PartialEq + 'static> GlobalMemo<T> {
    #[track_caller]
    /// Create a new global signal
    pub const fn new(selector: fn() -> T) -> GlobalMemo<T>
    where
        T: PartialEq,
    {
        let key = std::panic::Location::caller();
        GlobalMemo {
            selector,
            key: GlobalKey::new(key),
        }
    }

    /// Get the key for this global
    pub fn key(&self) -> GlobalKey<'static> {
        self.key.clone()
    }

    /// Get the signal that backs this global.
    pub fn memo(&self) -> Memo<T> {
        let key = self.key();

        let context = get_global_context();

        let read = context.signal.borrow();
        match read.get(&key) {
            Some(signal) => *signal.downcast_ref::<Memo<T>>().unwrap(),
            None => {
                drop(read);
                // Constructors are always run in the root scope
                let signal = ScopeId::ROOT.in_runtime(|| Signal::memo(self.selector));
                context.signal.borrow_mut().insert(key, Box::new(signal));
                signal
            }
        }
    }

    /// Get the scope the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        ScopeId::ROOT
    }

    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.memo().id()
    }
}

impl<T: PartialEq + 'static> Readable for GlobalMemo<T> {
    type Target = T;
    type Storage = UnsyncStorage;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.memo().try_read_unchecked()
    }

    #[track_caller]
    fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
        self.memo().peek_unchecked()
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
