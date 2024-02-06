use crate::read::Readable;
use crate::CopyValue;
use crate::Signal;
use crate::SignalData;
use dioxus_core::ScopeId;
use generational_box::Storage;
use std::fmt::Debug;
use std::fmt::Display;

/// A read only signal that has been mapped to a new type.
pub struct MappedSignal<U: 'static + ?Sized> {
    origin_scope: ScopeId,
    mapping: CopyValue<Box<dyn Fn() -> U>>,
}

impl MappedSignal<()> {
    /// Create a new mapped signal.
    pub fn new<T, S, U>(
        signal: Signal<T, S>,
        mapping: impl Fn(&T) -> &U + 'static,
    ) -> MappedSignal<S::Ref<U>>
    where
        S: Storage<SignalData<T>>,
        U: ?Sized,
    {
        MappedSignal {
            origin_scope: signal.origin_scope(),
            mapping: CopyValue::new(Box::new(move || S::map(signal.read(), &mapping))),
        }
    }
}

impl<U> MappedSignal<U> {
    /// Get the scope that the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.origin_scope
    }

    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    pub fn read(&self) -> U {
        (self.mapping.read())()
    }

    /// Run a closure with a reference to the signal's value.
    pub fn with<O>(&self, f: impl FnOnce(U) -> O) -> O {
        f(self.read())
    }
}

impl<U: ?Sized + Clone> MappedSignal<U> {
    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    pub fn value(&self) -> U {
        self.read().clone()
    }
}

impl<U: ?Sized> PartialEq for MappedSignal<U> {
    fn eq(&self, other: &Self) -> bool {
        self.mapping == other.mapping
    }
}

impl<U> std::clone::Clone for MappedSignal<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> Copy for MappedSignal<U> {}

impl<U: Display> Display for MappedSignal<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| Display::fmt(&v, f))
    }
}

impl<U: Debug> Debug for MappedSignal<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| Debug::fmt(&v, f))
    }
}

impl<T> std::ops::Deref for MappedSignal<T> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
        let uninit_callable = std::mem::MaybeUninit::<Self>::uninit();
        // Then move that value into the closure. We assume that the closure now has a in memory layout of Self.
        let uninit_closure = move || Self::read(unsafe { &*uninit_callable.as_ptr() });

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
            // We transmute self into a reference to the closure. This is safe because we know that the closure has the same memory layout as Self so &Closure == &Self.
            unsafe { std::mem::transmute(self) },
        );

        // Cast the closure to a trait object.
        reference_to_closure as &Self::Target
    }
}
