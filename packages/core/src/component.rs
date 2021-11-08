//! This file handles the supporting infrastructure for the `Component` trait and `Properties` which makes it possible
//! for components to be used within Nodes.
//!
//! Note - using the builder pattern does not required the Properties trait to be implemented - the only thing that matters is
//! if the type supports PartialEq. The Properties trait is used by the rsx! and html! macros to generate the type-safe builder
//! that ensures compile-time required and optional fields on cx.

use crate::innerlude::{Context, Element, LazyNodes, ScopeChildren};

pub struct FragmentProps<'a> {
    children: ScopeChildren<'a>,
}

pub struct FragmentBuilder<'a, const BUILT: bool> {
    children: Option<ScopeChildren<'a>>,
}
impl<'a> FragmentBuilder<'a, false> {
    pub fn children(self, children: ScopeChildren<'a>) -> FragmentBuilder<'a, true> {
        FragmentBuilder {
            children: Some(children),
        }
    }
}

impl<'a, const A: bool> FragmentBuilder<'a, A> {
    pub fn build(self) -> FragmentProps<'a> {
        FragmentProps {
            children: self.children.unwrap_or_default(),
        }
    }
}

impl<'a> Properties for FragmentProps<'a> {
    type Builder = FragmentBuilder<'a, false>;

    const IS_STATIC: bool = false;

    fn builder() -> Self::Builder {
        FragmentBuilder { children: None }
    }

    unsafe fn memoize(&self, _other: &Self) -> bool {
        false
    }
}

/// Create inline fragments using Component syntax.
///
/// Fragments capture a series of children without rendering extra nodes.
///
/// # Example
///
/// ```rust
/// rsx!{
///     Fragment { key: "abc" }
/// }
/// ```
///
/// # Details
///
/// Fragments are incredibly useful when necessary, but *do* add cost in the diffing phase.
/// Try to avoid nesting fragments if you can. There is no protection against infinitely nested fragments.
///
/// This function defines a dedicated `Fragment` component that can be used to create inline fragments in the RSX macro.
///
/// You want to use this free-function when your fragment needs a key and simply returning multiple nodes from rsx! won't cut it.
///
#[allow(non_upper_case_globals, non_snake_case)]
pub fn Fragment<'a>(cx: Context<'a>, props: &'a FragmentProps<'a>) -> Element {
    cx.render(Some(LazyNodes::new(|f| {
        f.fragment_from_iter(&props.children)
    })))
}

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
/// ```rust ignore
/// #[derive(Props, PartialEq)]
/// struct MyProps {
///     data: String
/// }
/// ```
///
/// For props that borrow:
///
/// ```rust ignore
/// #[derive(Props)]
/// struct MyProps<'a >{
///     data: &'a str
/// }
/// ```
pub trait Properties: Sized {
    type Builder;
    const IS_STATIC: bool;
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
    #[inline]
    pub fn build(self) {}
}

/// This utility function launches the builder method so rsx! and html! macros can use the typed-builder pattern
/// to initialize a component's props.
pub fn fc_to_builder<'a, T: Properties + 'a>(_: fn(Context<'a>, &T) -> Element) -> T::Builder {
    // pub fn fc_to_builder<'a, T: Properties + 'a>(_: fn(Scope<'a, T>) -> Element) -> T::Builder {
    T::builder()
}
