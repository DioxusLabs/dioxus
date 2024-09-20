use crate::innerlude::*;

/// Create inline fragments using Component syntax.
///
/// ## Details
///
/// Fragments capture a series of children without rendering extra nodes.
///
/// Creating fragments explicitly with the Fragment component is particularly useful when rendering lists or tables and
/// a key is needed to identify each item.
///
/// ## Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// let value = 1;
/// rsx! {
///     Fragment { key: "{value}" }
/// };
/// ```
///
/// ## Usage
///
/// Fragments are incredibly useful when necessary, but *do* add cost in the diffing phase.
/// Try to avoid highly nested fragments if you can. Unlike React, there is no protection against infinitely nested fragments.
///
/// This function defines a dedicated `Fragment` component that can be used to create inline fragments in the RSX macro.
///
/// You want to use this free-function when your fragment needs a key and simply returning multiple nodes from rsx! won't cut it.
#[allow(non_upper_case_globals, non_snake_case)]
pub fn Fragment(cx: FragmentProps) -> Element {
    cx.0
}

#[derive(Clone, PartialEq)]
pub struct FragmentProps(pub(crate) Element);

pub struct FragmentBuilder<const BUILT: bool>(Element);
impl FragmentBuilder<false> {
    pub fn children(self, children: Element) -> FragmentBuilder<true> {
        FragmentBuilder(children)
    }
}
impl<const A: bool> FragmentBuilder<A> {
    pub fn build(self) -> FragmentProps {
        FragmentProps(self.0)
    }
}

/// Access the children elements passed into the component
///
/// This enables patterns where a component is passed children from its parent.
///
/// ## Details
///
/// Unlike React, Dioxus allows *only* lists of children to be passed from parent to child - not arbitrary functions
/// or classes. If you want to generate nodes instead of accepting them as a list, consider declaring a closure
/// on the props that takes Context.
///
/// If a parent passes children into a component, the child will always re-render when the parent re-renders. In other
/// words, a component cannot be automatically memoized if it borrows nodes from its parent, even if the component's
/// props are valid for the static lifetime.
///
/// ## Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// fn app() -> Element {
///     rsx! {
///         CustomCard {
///             h1 {}
///             p {}
///         }
///     }
/// }
///
/// #[component]
/// fn CustomCard(children: Element) -> Element {
///     rsx! {
///         div {
///             h1 {"Title card"}
///             {children}
///         }
///     }
/// }
/// ```
impl Properties for FragmentProps {
    type Builder = FragmentBuilder<false>;
    fn builder() -> Self::Builder {
        FragmentBuilder(VNode::empty())
    }
    fn memoize(&mut self, new: &Self) -> bool {
        let equal = self == new;
        if !equal {
            let new_clone = new.clone();
            self.0 = new_clone.0;
        }
        equal
    }
}
