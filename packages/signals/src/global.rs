use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    mem::MaybeUninit,
    ops::Deref,
    rc::Rc,
};

use dioxus_core::{
    prelude::{provide_context, try_consume_context, IntoAttributeValue},
    ScopeId,
};
use generational_box::{GenerationalRef, GenerationalRefMut};

use crate::{selector, MappedSignal, ReadOnlySignal, Signal, Write};

/// A signal that can be accessed from anywhere in the application and created in a static
pub struct GlobalSignal<T> {
    initializer: fn() -> T,
}

#[derive(Clone)]
struct GlobalSignalContext {
    signal: Rc<RefCell<HashMap<*const (), Box<dyn Any>>>>,
}

fn get_global_context() -> GlobalSignalContext {
    match try_consume_context() {
        Some(context) => context,
        None => {
            let context = GlobalSignalContext {
                signal: Rc::new(RefCell::new(HashMap::new())),
            };
            provide_context(context)
        }
    }
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
                let value = ScopeId::ROOT.in_runtime(self.initializer);
                let signal = Signal::new(value);
                context.signal.borrow_mut().insert(key, Box::new(signal));
                signal
            }
        }
    }

    /// Get the scope the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        ScopeId::ROOT
    }

    /// Get the current value of the signal. This will subscribe the current scope to the signal.  If you would like to read the signal without subscribing to it, you can use [`Self::peek`] instead.
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn read(&self) -> GenerationalRef<T, Ref<'static, T>> {
        self.signal().read()
    }

    /// Get the current value of the signal. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    pub fn peek(&self) -> GenerationalRef<T, Ref<'static, T>> {
        self.signal().peek()
    }

    /// Get a mutable reference to the signal's value.
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn write(&self) -> Write<T, GenerationalRefMut<T, RefMut<'static, T>>> {
        self.signal().write()
    }

    /// Set the value of the signal. This will trigger an update on all subscribers.
    #[track_caller]
    pub fn set(&self, value: T) {
        self.signal().set(value);
    }

    /// Set the value of the signal without triggering an update on subscribers.
    #[track_caller]
    pub fn set_untracked(&self, value: T) {
        self.signal().set_untracked(value);
    }

    /// Run a closure with a reference to the signal's value.
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.signal().with(f)
    }

    /// Run a closure with a mutable reference to the signal's value.
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn with_mut<O>(&self, f: impl FnOnce(&mut T) -> O) -> O {
        self.signal().with_mut(f)
    }

    /// Map the signal to a new type.
    pub fn map<O>(
        &self,
        f: impl Fn(&T) -> &O + 'static,
    ) -> MappedSignal<GenerationalRef<O, Ref<'static, O>>> {
        MappedSignal::new(self.signal(), f)
    }

    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.signal().id()
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

impl<T: Clone + 'static> GlobalSignal<T> {
    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn cloned(&self) -> T {
        self.read().clone()
    }
}

impl GlobalSignal<bool> {
    /// Invert the boolean value of the signal. This will trigger an update on all subscribers.
    pub fn toggle(&self) {
        self.set(!self.cloned());
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

/// A signal that can be accessed from anywhere in the application and created in a static
pub struct GlobalSelector<T: 'static> {
    selector: fn() -> T,
}

impl<T: PartialEq + 'static> GlobalSelector<T> {
    /// Create a new global signal
    pub const fn new(selector: fn() -> T) -> GlobalSelector<T>
    where
        T: PartialEq,
    {
        GlobalSelector { selector }
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
                let signal = ScopeId::ROOT.in_runtime(|| Signal::selector(self.selector));
                context.signal.borrow_mut().insert(key, Box::new(signal));
                signal
            }
        }
    }

    /// Get the scope the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        ScopeId::ROOT
    }

    /// Get the current value of the signal. This will subscribe the current scope to the signal.  If you would like to read the signal without subscribing to it, you can use [`Self::peek`] instead.
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn read(&self) -> GenerationalRef<T, Ref<'static, T>> {
        self.signal().read()
    }

    /// Get the current value of the signal. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    pub fn peek(&self) -> GenerationalRef<T, Ref<'static, T>> {
        self.signal().peek()
    }

    /// Run a closure with a reference to the signal's value.
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.signal().with(f)
    }

    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.signal().id()
    }
}

impl<T: PartialEq + 'static> IntoAttributeValue for GlobalSelector<T>
where
    T: Clone + IntoAttributeValue,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.signal().into_value()
    }
}

impl<T: PartialEq + Clone + 'static> GlobalSelector<T> {
    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn cloned(&self) -> T {
        self.read().clone()
    }
}

impl<T: PartialEq + 'static> PartialEq for GlobalSelector<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<T: PartialEq + Clone + 'static> Deref for GlobalSelector<T> {
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
