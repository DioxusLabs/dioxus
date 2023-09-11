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
/// ```rust, ignore
/// rsx!{
///     Fragment { key: "abc" }
/// }
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
pub fn Fragment<'a>(cx: Scope<'a, FragmentProps<'a>>) -> Element {
    let children = cx.props.0.as_ref()?;
    Some(VNode {
        key: children.key,
        parent: children.parent.clone(),
        stable_id: children.stable_id.clone(),
        template: children.template.clone(),
        root_ids: children.root_ids.clone(),
        dynamic_nodes: children.dynamic_nodes,
        dynamic_attrs: children.dynamic_attrs,
    })
}

pub struct FragmentProps<'a>(Element<'a>);
pub struct FragmentBuilder<'a, const BUILT: bool>(Element<'a>);
impl<'a> FragmentBuilder<'a, false> {
    pub fn children(self, children: Element<'a>) -> FragmentBuilder<'a, true> {
        FragmentBuilder(children)
    }
}
impl<'a, const A: bool> FragmentBuilder<'a, A> {
    pub fn build(self) -> FragmentProps<'a> {
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
/// ```rust, ignore
/// fn App(cx: Scope) -> Element {
///     cx.render(rsx!{
///         CustomCard {
///             h1 {}
///             p {}
///         }
///     })
/// }
///
/// #[derive(PartialEq, Props)]
/// struct CardProps {
///     children: Element
/// }
///
/// fn CustomCard(cx: Scope<CardProps>) -> Element {
///     cx.render(rsx!{
///         div {
///             h1 {"Title card"}
///             {cx.props.children}
///         }
///     })
/// }
/// ```
impl<'a> Properties for FragmentProps<'a> {
    type Builder = FragmentBuilder<'a, false>;
    const IS_STATIC: bool = false;
    fn builder() -> Self::Builder {
        FragmentBuilder(None)
    }
    unsafe fn memoize(&self, _other: &Self) -> bool {
        false
    }
}
