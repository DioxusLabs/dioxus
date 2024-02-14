use crate::{read::Readable, ReadableRef};
use dioxus_core::prelude::{IntoAttributeValue, ScopeId};
use generational_box::UnsyncStorage;
use std::{mem::MaybeUninit, ops::Deref};

use crate::{ReadOnlySignal, Signal};

use super::get_global_context;

/// A signal that can be accessed from anywhere in the application and created in a static
pub struct GlobalMemo<T: 'static> {
    selector: fn() -> T,
}

impl<T: PartialEq + 'static> GlobalMemo<T> {
    /// Create a new global signal
    pub const fn new(selector: fn() -> T) -> GlobalMemo<T>
    where
        T: PartialEq,
    {
        GlobalMemo { selector }
    }

    /// Get the signal that backs this global.
    pub fn signal(&self) -> ReadOnlySignal<T> {
        let key = self as *const _ as *const ();

        let context = get_global_context();

        let read = context.signal.borrow();
        match read.get(&key) {
            Some(signal) => *signal.downcast_ref::<ReadOnlySignal<T>>().unwrap(),
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
        self.signal().id()
    }
}

impl<T: PartialEq + 'static> Readable for GlobalMemo<T> {
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

impl<T: PartialEq + 'static> IntoAttributeValue for GlobalMemo<T>
where
    T: Clone + IntoAttributeValue,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.signal().into_value()
    }
}

impl<T: PartialEq + 'static> PartialEq for GlobalMemo<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<T: PartialEq + Clone + 'static> Deref for GlobalMemo<T> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
        let uninit_callable = MaybeUninit::<Self>::uninit();
        // Then move that value into the closure. We assume that the closure now has a in memory layout of Self.
        let uninit_closure = move || Self::read(unsafe { &*uninit_callable.as_ptr() }).clone();

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
