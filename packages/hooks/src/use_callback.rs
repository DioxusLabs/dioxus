use dioxus_core::prelude::use_hook;
use dioxus_signals::CopyValue;
use dioxus_signals::Writable;

/// A callback that's always current
///
/// Whenever this hook is called the inner callback will be replaced with the new callback but the handle will remain.
///
/// There is *currently* no signal tracking on the Callback so anything reading from it will not be updated.
///
/// This API is in flux and might not remain.
pub fn use_callback<O>(f: impl FnMut() -> O + 'static) -> UseCallback<O> {
    // Create a copyvalue with no contents
    // This copyvalue is generic over F so that it can be sized properly
    let mut inner = use_hook(|| CopyValue::new(None));

    // Every time this hook is called replace the inner callback with the new callback
    inner.set(Some(f));

    // And then wrap that callback in a boxed callback so we're blind to the size of the actual callback
    use_hook(|| UseCallback {
        inner: CopyValue::new(Box::new(move || {
            inner.with_mut(|f: &mut Option<_>| f.as_mut().unwrap()())
        })),
    })
}

/// This callback is not generic over a return type so you can hold a bunch of callbacks at once
///
/// If you need a callback that returns a value, you can simply wrap the closure you pass in that sets a value in its scope
#[derive(PartialEq)]
pub struct UseCallback<O: 'static + ?Sized> {
    inner: CopyValue<Box<dyn FnMut() -> O>>,
}

impl<O: 'static + ?Sized> Clone for UseCallback<O> {
    fn clone(&self) -> Self {
        Self { inner: self.inner }
    }
}
impl<O: 'static> Copy for UseCallback<O> {}

impl<O> UseCallback<O> {
    /// Call the callback
    pub fn call(&mut self) -> O {
        self.inner.with_mut(|f| f())
    }
}
