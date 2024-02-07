use crate::write::Writable;
use crate::Write;
use crate::{read::Readable, ReadableRef};
use dioxus_core::prelude::{IntoAttributeValue, ScopeId};
use generational_box::UnsyncStorage;
use std::{mem::MaybeUninit, ops::Deref};

use super::get_global_context;
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
    pub fn write(&self) -> Write<T, UnsyncStorage> {
        self.signal().write()
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
    fn try_read(&self) -> Result<ReadableRef<Self>, generational_box::BorrowError> {
        self.signal().try_read()
    }

    #[track_caller]
    fn peek(&self) -> ReadableRef<Self> {
        self.signal().peek()
    }
}

impl<T: 'static> Writable for GlobalSignal<T> {
    type Mut<R: ?Sized + 'static> = Write<R, UnsyncStorage>;

    fn map_mut<I: ?Sized, U: ?Sized + 'static, F: FnOnce(&mut I) -> &mut U>(
        ref_: Self::Mut<I>,
        f: F,
    ) -> Self::Mut<U> {
        Write::map(ref_, f)
    }

    fn try_map_mut<
        I: ?Sized + 'static,
        U: ?Sized + 'static,
        F: FnOnce(&mut I) -> Option<&mut U>,
    >(
        ref_: Self::Mut<I>,
        f: F,
    ) -> Option<Self::Mut<U>> {
        Write::filter_map(ref_, f)
    }

    #[track_caller]
    fn try_write(&self) -> Result<Self::Mut<T>, generational_box::BorrowMutError> {
        self.signal().try_write()
    }
}

impl<T: 'static> IntoAttributeValue for GlobalSignal<T>
where
    T: Clone + IntoAttributeValue,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.signal().into_value()
    }
}

impl<T: 'static> PartialEq for GlobalSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<T: Clone + 'static> Deref for GlobalSignal<T> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
        let uninit_callable = MaybeUninit::<Self>::uninit();

        // Then move that value into the closure. We assume that the closure now has a in memory layout of Self.
        let uninit_closure = move || {
            <GlobalSignal<T> as Readable>::read(unsafe { &*uninit_callable.as_ptr() }).clone()
        };

        // Check that the size of the closure is the same as the size of Self in case the compiler changed the layout of the closure.
        let size_of_closure = std::mem::size_of_val(&uninit_closure);
        assert_eq!(size_of_closure, std::mem::size_of::<Self>());

        // Then cast the lifetime of the closure to the lifetime of &self.
        fn cast_lifetime<'a, T>(_a: &T, b: &'a T) -> &'a T {
            b
        }
        let reference_to_closure = cast_lifetime(
            {
                // The real closure that we will never use.
                &uninit_closure
            },
            // We transmute self into a reference to the closure. This is safe because we know that the closure has the same memory layout as Self so &Closure == &self.
            unsafe { std::mem::transmute(self) },
        );

        // Cast the closure to a trait object.
        reference_to_closure as &Self::Target
    }
}
