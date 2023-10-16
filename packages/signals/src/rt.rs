use std::cell::{Ref, RefMut};

use std::mem::MaybeUninit;
use std::ops::Deref;
use std::rc::Rc;

use dioxus_core::prelude::*;
use dioxus_core::ScopeId;

use generational_box::{GenerationalBox, Owner, Store};

use crate::Effect;

fn current_store() -> Store {
    match consume_context() {
        Some(rt) => rt,
        None => {
            let store = Store::default();
            provide_root_context(store).expect("in a virtual dom")
        }
    }
}

fn current_owner() -> Rc<Owner> {
    match Effect::current() {
        // If we are inside of an effect, we should use the owner of the effect as the owner of the value.
        Some(effect) => {
            let scope_id = effect.source;
            owner_in_scope(scope_id)
        }
        // Otherwise either get an owner from the current scope or create a new one.
        None => match has_context() {
            Some(rt) => rt,
            None => {
                let owner = Rc::new(current_store().owner());
                provide_context(owner).expect("in a virtual dom")
            }
        },
    }
}

fn owner_in_scope(scope: ScopeId) -> Rc<Owner> {
    match consume_context_from_scope(scope) {
        Some(rt) => rt,
        None => {
            let owner = Rc::new(current_store().owner());
            provide_context_to_scope(scope, owner).expect("in a virtual dom")
        }
    }
}

/// CopyValue is a wrapper around a value to make the value mutable and Copy.
///
/// It is internally backed by [`generational_box::GenerationalBox`].
pub struct CopyValue<T: 'static> {
    pub(crate) value: GenerationalBox<T>,
    origin_scope: ScopeId,
}

#[cfg(feature = "serde")]
impl<T: 'static> serde::Serialize for CopyValue<T>
where
    T: serde::Serialize,
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.read().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: 'static> serde::Deserialize<'de> for CopyValue<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = T::deserialize(deserializer)?;

        Ok(Self::new(value))
    }
}

impl<T: 'static> CopyValue<T> {
    /// Create a new CopyValue. The value will be stored in the current component.
    ///
    /// Once the component this value is created in is dropped, the value will be dropped.
    pub fn new(value: T) -> Self {
        let owner = current_owner();

        Self {
            value: owner.insert(value),
            origin_scope: current_scope_id().expect("in a virtual dom"),
        }
    }

    /// Create a new CopyValue. The value will be stored in the given scope. When the specified scope is dropped, the value will be dropped.
    pub fn new_in_scope(value: T, scope: ScopeId) -> Self {
        let owner = owner_in_scope(scope);

        Self {
            value: owner.insert(value),
            origin_scope: scope,
        }
    }

    pub(crate) fn invalid() -> Self {
        let owner = current_owner();

        Self {
            value: owner.invalid(),
            origin_scope: current_scope_id().expect("in a virtual dom"),
        }
    }

    /// Get the scope this value was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.origin_scope
    }

    /// Try to read the value. If the value has been dropped, this will return None.
    pub fn try_read(&self) -> Option<Ref<'_, T>> {
        self.value.try_read()
    }

    /// Read the value. If the value has been dropped, this will panic.
    pub fn read(&self) -> Ref<'static, T> {
        self.value.read()
    }

    /// Try to write the value. If the value has been dropped, this will return None.
    pub fn try_write(&self) -> Option<RefMut<'static, T>> {
        self.value.try_write()
    }

    /// Write the value. If the value has been dropped, this will panic.
    pub fn write(&self) -> RefMut<'static, T> {
        self.value.write()
    }

    /// Set the value. If the value has been dropped, this will panic.
    pub fn set(&mut self, value: T) {
        *self.write() = value;
    }

    /// Run a function with a reference to the value. If the value has been dropped, this will panic.
    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        let write = self.read();
        f(&*write)
    }

    /// Run a function with a mutable reference to the value. If the value has been dropped, this will panic.
    pub fn with_mut<O>(&self, f: impl FnOnce(&mut T) -> O) -> O {
        let mut write = self.write();
        f(&mut *write)
    }
}

impl<T: Clone + 'static> CopyValue<T> {
    /// Get the value. If the value has been dropped, this will panic.
    pub fn value(&self) -> T {
        self.read().clone()
    }
}

impl<T: 'static> PartialEq for CopyValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value.ptr_eq(&other.value)
    }
}

impl<T> Deref for CopyValue<T> {
    type Target = dyn Fn() -> Ref<'static, T>;

    fn deref(&self) -> &Self::Target {
        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
        let uninit_callable = MaybeUninit::<Self>::uninit();
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
