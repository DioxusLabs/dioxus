use crate::innerlude::*;

/// Every "Props" used for a component must implement the `Properties` trait. This trait gives some hints to Dioxus
/// on how to memoize the props and some additional optimizations that can be made. We strongly encourage using the
/// derive macro to implement the `Properties` trait automatically as guarantee that your memoization strategy is safe.
///
/// If your props are 'static, then Dioxus will require that they also be PartialEq for the derived memoize strategy. However,
/// if your props borrow data, then the memoization strategy will simply default to "false" and the PartialEq will be ignored.
/// This tends to be useful when props borrow something that simply cannot be compared (IE a reference to a closure);
///
/// By default, the memoization strategy is very conservative, but can be tuned to be more aggressive manually. However,
/// this is only safe if the props are 'static - otherwise you might borrow references after-free.
///
/// We strongly suggest that any changes to memoization be done at the "PartialEq" level for 'static props. Additionally,
/// we advise the use of smart pointers in cases where memoization is important.
///
/// ## Example
///
/// For props that are 'static:
/// ```rust, ignore
/// #[derive(Props, PartialEq)]
/// struct MyProps {
///     data: String
/// }
/// ```
///
/// For props that borrow:
///
/// ```rust, ignore
/// #[derive(Props)]
/// struct MyProps<'a >{
///     data: &'a str
/// }
/// ```
pub trait Properties: Sized {
    /// The type of the builder for this component.
    /// Used to create "in-progress" versions of the props.
    type Builder;

    /// An indication if these props are can be memoized automatically.
    const IS_STATIC: bool;

    /// Create a builder for this component.
    fn builder() -> Self::Builder;

    /// Memoization can only happen if the props are valid for the 'static lifetime
    ///
    /// # Safety
    /// The user must know if their props are static, but if they make a mistake, UB happens
    /// Therefore it's unsafe to memoize.
    unsafe fn memoize(&self, other: &Self) -> bool;
}

impl Properties for () {
    type Builder = EmptyBuilder;
    const IS_STATIC: bool = true;
    fn builder() -> Self::Builder {
        EmptyBuilder {}
    }
    unsafe fn memoize(&self, _other: &Self) -> bool {
        true
    }
}
// We allow components to use the () generic parameter if they have no props. This impl enables the "build" method
// that the macros use to anonymously complete prop construction.
pub struct EmptyBuilder;
impl EmptyBuilder {
    pub fn build(self) {}
}

/// This utility function launches the builder method so rsx! and html! macros can use the typed-builder pattern
/// to initialize a component's props.
pub fn fc_to_builder<'a, T: Properties + 'a>(_: fn(Scope<'a, T>) -> Element<'a>) -> T::Builder {
    T::builder()
}

#[cfg(not(miri))]
#[test]
fn unsafe_props_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("compile_tests/props_safety.rs");
    t.compile_fail("compile_tests/props_safety_temporary_values.rs");
}
