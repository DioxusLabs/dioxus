use dioxus_core::ScopeId;
use generational_box::BorrowResult;
use std::{any::Any, cell::RefCell, collections::HashMap, ops::Deref, panic::Location, rc::Rc};

mod memo;
pub use memo::*;

mod signal;
pub use signal::*;

use crate::{Readable, ReadableRef, Signal, Writable, WritableRef};

/// A trait for an item that can be constructed from an initialization function
pub trait InitializeFromFunction<T> {
    /// Create an instance of this type from an initialization function
    fn initialize_from_function(f: fn() -> T) -> Self;
}

impl<T> InitializeFromFunction<T> for T {
    fn initialize_from_function(f: fn() -> T) -> Self {
        f()
    }
}

/// A lazy value that is created once per application and can be accessed from anywhere in that application
pub struct Global<T, R = T> {
    constructor: fn() -> R,
    key: GlobalKey<'static>,
    phantom: std::marker::PhantomData<fn() -> T>,
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<T: Clone + 'static, R: Clone + 'static> Deref for Global<T, R>
where
    T: Readable<Target = R> + InitializeFromFunction<R>,
{
    type Target = dyn Fn() -> R;

    fn deref(&self) -> &Self::Target {
        unsafe { Readable::deref_impl(self) }
    }
}

impl<T: Clone + 'static, R: 'static> Readable for Global<T, R>
where
    T: Readable<Target = R> + InitializeFromFunction<R>,
{
    type Target = R;
    type Storage = T::Storage;

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

impl<T: Clone + 'static, R: 'static> Writable for Global<T, R>
where
    T: Writable<Target = R> + InitializeFromFunction<R>,
{
    type Mut<'a, Read: ?Sized + 'static> = T::Mut<'a, Read>;

    fn map_mut<I: ?Sized, U: ?Sized + 'static, F: FnOnce(&mut I) -> &mut U>(
        ref_: Self::Mut<'_, I>,
        f: F,
    ) -> Self::Mut<'_, U> {
        T::map_mut(ref_, f)
    }

    fn try_map_mut<
        I: ?Sized + 'static,
        U: ?Sized + 'static,
        F: FnOnce(&mut I) -> Option<&mut U>,
    >(
        ref_: Self::Mut<'_, I>,
        f: F,
    ) -> Option<Self::Mut<'_, U>> {
        T::try_map_mut(ref_, f)
    }

    fn downcast_lifetime_mut<'a: 'b, 'b, Read: ?Sized + 'static>(
        mut_: Self::Mut<'a, Read>,
    ) -> Self::Mut<'b, Read> {
        T::downcast_lifetime_mut(mut_)
    }

    #[track_caller]
    fn try_write_unchecked(
        &self,
    ) -> Result<WritableRef<'static, Self>, generational_box::BorrowMutError> {
        self.resolve().try_write_unchecked()
    }
}

impl<T: Clone + 'static, R: 'static> Global<T, R>
where
    T: Writable<Target = R> + InitializeFromFunction<R>,
{
    /// Write this value
    pub fn write(&self) -> T::Mut<'static, R> {
        self.resolve().try_write_unchecked().unwrap()
    }

    /// Run a closure with a mutable reference to the signal's value.
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn with_mut<O>(&self, f: impl FnOnce(&mut R) -> O) -> O {
        self.resolve().with_mut(f)
    }
}

impl<T: Clone + 'static, R> Global<T, R>
where
    T: InitializeFromFunction<R>,
{
    #[track_caller]
    /// Create a new global value
    pub const fn new(constructor: fn() -> R) -> Self {
        let key = std::panic::Location::caller();
        Self {
            constructor,
            key: GlobalKey::new(key),
            phantom: std::marker::PhantomData,
        }
    }

    /// Create this global signal with a specific key.
    /// This is useful for ensuring that the signal is unique across the application and accessible from
    /// outside the application too.
    #[track_caller]
    pub const fn with_name(constructor: fn() -> R, key: &'static str) -> Self {
        Self {
            constructor,
            key: GlobalKey::File {
                file: key,
                line: 0,
                column: 0,
                index: 0,
            },
            phantom: std::marker::PhantomData,
        }
    }

    /// Create this global signal with a specific key.
    /// This is useful for ensuring that the signal is unique across the application and accessible from
    /// outside the application too.
    #[track_caller]
    pub const fn with_location(
        constructor: fn() -> R,
        file: &'static str,
        line: u32,
        column: u32,
        index: usize,
    ) -> Self {
        Self {
            constructor,
            key: GlobalKey::File {
                file,
                line: line as _,
                column: column as _,
                index: index as _,
            },
            phantom: std::marker::PhantomData,
        }
    }

    /// Get the key for this global
    pub fn key(&self) -> GlobalKey<'static> {
        self.key.clone()
    }

    /// Resolve the global value. This will try to get the existing value from the current virtual dom, and if it doesn't exist, it will create a new one.
    // NOTE: This is not called "get" or "value" because those methods overlap with Readable and Writable
    pub fn resolve(&self) -> T {
        let key = self.key();

        let context = get_global_context();

        // Get the entry if it already exists
        {
            let read = context.map.borrow();
            if let Some(signal) = read.get(&key) {
                return signal.downcast_ref::<T>().cloned().unwrap();
            }
        }
        // Otherwise, create it
        // Constructors are always run in the root scope
        let signal = ScopeId::ROOT.in_runtime(|| T::initialize_from_function(self.constructor));
        context
            .map
            .borrow_mut()
            .insert(key, Box::new(signal.clone()));
        signal
    }

    /// Get the scope the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        ScopeId::ROOT
    }
}

/// The context for global signals
#[derive(Clone, Default)]
pub struct GlobalLazyContext {
    map: Rc<RefCell<HashMap<GlobalKey<'static>, Box<dyn Any>>>>,
}

/// A key used to identify a signal in the global signal context
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GlobalKey<'a> {
    /// A key derived from a `std::panic::Location` type
    File {
        /// The file name
        file: &'a str,

        /// The line number
        line: u32,

        /// The column number
        column: u32,

        /// The index of the signal in the file - used to disambiguate macro calls
        index: u32,
    },

    /// A raw key derived just from a string
    Raw(&'a str),
}

impl<'a> GlobalKey<'a> {
    /// Create a new key from a location
    pub const fn new(key: &'a Location<'a>) -> Self {
        GlobalKey::File {
            file: key.file(),
            line: key.line(),
            column: key.column(),
            index: 0,
        }
    }
}

impl From<&'static Location<'static>> for GlobalKey<'static> {
    fn from(key: &'static Location<'static>) -> Self {
        Self::new(key)
    }
}

impl GlobalLazyContext {
    /// Get a signal with the given string key
    /// The key will be converted to a UUID with the appropriate internal namespace
    pub fn get_signal_with_key<T>(&self, key: GlobalKey) -> Option<Signal<T>> {
        self.map.borrow().get(&key).map(|f| {
            *f.downcast_ref::<Signal<T>>().unwrap_or_else(|| {
                panic!(
                    "Global signal with key {:?} is not of the expected type. Keys are {:?}",
                    key,
                    self.map.borrow().keys()
                )
            })
        })
    }
}

/// Get the global context for signals
pub fn get_global_context() -> GlobalLazyContext {
    match ScopeId::ROOT.has_context() {
        Some(context) => context,
        None => ScopeId::ROOT.provide_context(Default::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that keys of global signals are correctly generated and different from one another.
    /// We don't want signals to merge, but we also want them to use both string IDs and memory addresses.
    #[test]
    fn test_global_keys() {
        // we're using consts since it's harder than statics due to merging - these won't be merged
        const MYSIGNAL: GlobalSignal<i32> = GlobalSignal::new(|| 42);
        const MYSIGNAL2: GlobalSignal<i32> = GlobalSignal::new(|| 42);
        const MYSIGNAL3: GlobalSignal<i32> = GlobalSignal::with_name(|| 42, "custom-keyed");

        let a = MYSIGNAL.key();
        let b = MYSIGNAL.key();
        let c = MYSIGNAL.key();
        assert_eq!(a, b);
        assert_eq!(b, c);

        let d = MYSIGNAL2.key();
        assert_ne!(a, d);

        let e = MYSIGNAL3.key();
        assert_ne!(a, e);
    }
}
