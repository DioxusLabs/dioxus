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
    cx.children
}

/// Props for the [`Fragment`] component.
///
/// `children` are the nodes captured by the fragment. A component cannot be automatically memoized
/// if it borrows nodes from its parent, so a fragment always re-renders when its parent does.
#[derive(dioxus_core_macro::Props, Clone, PartialEq)]
pub struct FragmentProps {
    children: Element,
}
