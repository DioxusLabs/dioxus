use crate::write::Writable;
use crate::{read::Readable, ReadableRef};
use crate::{WritableRef, Write};
use dioxus_core::prelude::ScopeId;
use generational_box::UnsyncStorage;
use std::ops::Deref;

use super::get_global_context;
use crate::read_impls;
use crate::Signal;

/// A signal that can be accessed from anywhere in the application and created in a static
pub struct GlobalSignal<T> {
    initializer: fn() -> T,
}

impl<T: 'static> GlobalSignal<T> {
    /// Create a new global signal with the given initializer.
    pub const fn new(initializer: fn() -> T) -> GlobalSignal<T> {
        GlobalSignal { initializer }
    }

    /// Get the signal that backs this global.
    pub fn signal(&self) -> Signal<T> {
        let key = self as *const _ as *const ();
        let context = get_global_context();
        let read = context.signal.borrow();

        match read.get(&key) {
            Some(signal) => *signal.downcast_ref::<Signal<T>>().unwrap(),
            None => {
                drop(read);

                // Constructors are always run in the root scope
                // The signal also exists in the root scope
                let value = ScopeId::ROOT.in_runtime(self.initializer);
                let signal = Signal::new_in_scope(value, ScopeId::ROOT);

                let entry = context.signal.borrow_mut().insert(key, Box::new(signal));
                debug_assert!(entry.is_none(), "Global signal already exists");

                signal
            }
        }
    }

    /// Write this value
    pub fn write(&self) -> Write<'static, T, UnsyncStorage> {
        self.signal().try_write_unchecked().unwrap()
    }

    /// Get the scope the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        ScopeId::ROOT
    }

    /// Run a closure with a mutable reference to the signal's value.
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn with_mut<O>(&self, f: impl FnOnce(&mut T) -> O) -> O {
        self.signal().with_mut(f)
    }

    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.signal().id()
    }
}

impl<T: 'static> Readable for GlobalSignal<T> {
    type Target = T;
    type Storage = UnsyncStorage;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.signal().try_read_unchecked()
    }

    #[track_caller]
    fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
        self.signal().peek_unchecked()
    }
}

impl<T: 'static> Writable for GlobalSignal<T> {
    type Mut<'a, R: ?Sized + 'static> = Write<'a, R, UnsyncStorage>;

    fn map_mut<I: ?Sized, U: ?Sized + 'static, F: FnOnce(&mut I) -> &mut U>(
        ref_: Self::Mut<'_, I>,
        f: F,
    ) -> Self::Mut<'_, U> {
        Write::map(ref_, f)
    }

    fn try_map_mut<
        I: ?Sized + 'static,
        U: ?Sized + 'static,
        F: FnOnce(&mut I) -> Option<&mut U>,
    >(
        ref_: Self::Mut<'_, I>,
        f: F,
    ) -> Option<Self::Mut<'_, U>> {
        Write::filter_map(ref_, f)
    }

    fn downcast_lifetime_mut<'a: 'b, 'b, R: ?Sized + 'static>(
        mut_: Self::Mut<'a, R>,
    ) -> Self::Mut<'b, R> {
        Write::downcast_lifetime(mut_)
    }

    #[track_caller]
    fn try_write_unchecked(
        &self,
    ) -> Result<WritableRef<'static, Self>, generational_box::BorrowMutError> {
        self.signal().try_write_unchecked()
    }
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<T: Clone + 'static> Deref for GlobalSignal<T> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        Readable::deref_impl(self)
    }
}

read_impls!(GlobalSignal<T>);
