use std::rc::Rc;

use dioxus_core::prelude::*;

/// Store constant state between component renders.
///
/// UseConst allows you to store state that is initialized once and then remains constant across renders.
/// You can only get an immutable reference after initalization.
/// This can be useful for values that don't need to update reactively, thus can be memoized easily
///
/// ```rust, ignore
/// struct ComplexData(i32);
///
/// fn Component(cx: Scope) -> Element {
///   let id = use_const(cx, || ComplexData(100));
///
///   cx.render(rsx! {
///     div { "{id.0}" }
///   })
/// }
/// ```
#[must_use]
pub fn use_const<T: 'static>(
    cx: &ScopeState,
    initial_state_fn: impl FnOnce() -> T,
) -> &UseConst<T> {
    cx.use_hook(|| UseConst {
        value: Rc::new(initial_state_fn()),
    })
}

#[derive(Clone)]
pub struct UseConst<T> {
    value: Rc<T>,
}

impl<T> PartialEq for UseConst<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.value, &other.value)
    }
}

impl<T: core::fmt::Display> core::fmt::Display for UseConst<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl<T> UseConst<T> {
    pub fn get_rc(&self) -> &Rc<T> {
        &self.value
    }
}

impl<T> std::ops::Deref for UseConst<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref()
    }
}

#[test]
fn use_const_makes_sense() {
    #[allow(unused)]

    fn app(cx: Scope) -> Element {
        let const_val = use_const(cx, || vec![0, 1, 2, 3]);

        assert!(const_val[0] == 0);

        // const_val.remove(0); // Cannot Compile, cannot get mutable reference now

        None
    }
}
