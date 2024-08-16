use crate::write::Writable;
use crate::{read::Readable, ReadableRef};
use crate::{WritableRef, Write};
use generational_box::{BorrowResult, UnsyncStorage};
use std::ops::Deref;

use super::{InitializeFromFunction, LazyGlobal};
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
    /// Write this value
    pub fn write(&self) -> Write<'static, T, UnsyncStorage> {
        self.resolve().try_write_unchecked().unwrap()
    }

    /// Run a closure with a mutable reference to the signal's value.
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn with_mut<O>(&self, f: impl FnOnce(&mut T) -> O) -> O {
        self.resolve().with_mut(f)
    }

    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.resolve().id()
    }
}

impl<T: 'static> Readable for GlobalSignal<T> {
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
        self.resolve().try_write_unchecked()
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
