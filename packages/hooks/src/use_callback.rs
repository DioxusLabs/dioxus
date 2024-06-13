use dioxus_core::prelude::{current_scope_id, use_hook, Runtime};
use dioxus_signals::CopyValue;
use dioxus_signals::Writable;

/// A callback that's always current
///
/// Whenever this hook is called the inner callback will be replaced with the new callback but the handle will remain.
///
/// There is *currently* no signal tracking on the Callback so anything reading from it will not be updated.
///
/// This API is in flux and might not remain.
#[doc = include_str!("../docs/rules_of_hooks.md")]
pub fn use_callback<O>(f: impl FnMut() -> O + 'static) -> UseCallback<O> {
    // Create a copyvalue with no contents
    // This copyvalue is generic over F so that it can be sized properly
    let mut inner = use_hook(|| CopyValue::new(None));

    // Every time this hook is called replace the inner callback with the new callback
    inner.set(Some(f));

    // And then wrap that callback in a boxed callback so we're blind to the size of the actual callback
    use_hook(|| {
        let cur_scope = current_scope_id().unwrap();
        let rt = Runtime::current().unwrap();

        UseCallback {
            inner: CopyValue::new(Box::new(move || {
                // run this callback in the context of the scope it was created in.
                let run_callback = || inner.with_mut(|f: &mut Option<_>| f.as_mut().unwrap()());
                rt.on_scope(cur_scope, run_callback)
            })),
        }
    })
}

/// This callback is not generic over a return type so you can hold a bunch of callbacks at once
///
/// If you need a callback that returns a value, you can simply wrap the closure you pass in that sets a value in its scope
pub struct UseCallback<O: 'static + ?Sized> {
    inner: CopyValue<Box<dyn FnMut() -> O>>,
}

impl<O: 'static + ?Sized> PartialEq for UseCallback<O> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<O: 'static + ?Sized> std::fmt::Debug for UseCallback<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UseCallback")
            .field("inner", &self.inner.value())
            .finish()
    }
}

impl<O: 'static + ?Sized> Clone for UseCallback<O> {
    fn clone(&self) -> Self {
        Self { inner: self.inner }
    }
}
impl<O: 'static> Copy for UseCallback<O> {}

impl<O> UseCallback<O> {
    /// Call the callback
    pub fn call(&self) -> O {
        (self.inner.write_unchecked())()
    }
}

// This makes UseCallback callable like a normal function
impl<O> std::ops::Deref for UseCallback<O> {
    type Target = dyn Fn() -> O;

    fn deref(&self) -> &Self::Target {
        use std::mem::MaybeUninit;

        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
        let uninit_callable = MaybeUninit::<Self>::uninit();
        // Then move that value into the closure. We assume that the closure now has a in memory layout of Self.
        let uninit_closure = move || Self::call(unsafe { &*uninit_callable.as_ptr() });

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
            #[allow(clippy::missing_transmute_annotations)]
            // We transmute self into a reference to the closure. This is safe because we know that the closure has the same memory layout as Self so &Closure == &Self.
            unsafe {
                std::mem::transmute(self)
            },
        );

        // Cast the closure to a trait object.
        reference_to_closure as &_
    }
}
