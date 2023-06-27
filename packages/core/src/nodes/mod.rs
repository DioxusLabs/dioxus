use crate::{Element, LazyNodes, ScopeState};

mod attributes;
pub use attributes::*;

mod boxed_cell_slice;
pub use boxed_cell_slice::*;

mod vnode;
pub use vnode::*;

mod template;
pub use template::*;

mod into_attribute;
pub use into_attribute::*;

mod into_vnode;
pub use into_vnode::*;

pub type TemplateId = &'static str;

/// The actual state of the component's most recent computation
///
/// Because Dioxus accepts components in the form of `async fn(Scope) -> Result<VNode>`, we need to support both
/// sync and async versions.
///
/// Dioxus will do its best to immediately resolve any async components into a regular Element, but as an implementor
/// you might need to handle the case where there's no node immediately ready.
pub enum RenderReturn<'a> {
    /// A currently-available element
    Ready(VNode<'a>),

    /// The component aborted rendering early. It might've thrown an error.
    ///
    /// In its place we've produced a placeholder to locate its spot in the dom when
    /// it recovers.
    Aborted(VPlaceholder),
}

impl<'a> Default for RenderReturn<'a> {
    fn default() -> Self {
        RenderReturn::Aborted(VPlaceholder::default())
    }
}

impl<'a> RenderReturn<'a> {
    pub(crate) unsafe fn extend_lifetime_ref<'c>(&self) -> &'c RenderReturn<'c> {
        unsafe { std::mem::transmute(self) }
    }
    pub(crate) unsafe fn extend_lifetime<'c>(self) -> RenderReturn<'c> {
        unsafe { std::mem::transmute(self) }
    }
}

pub trait IntoTemplate<'a> {
    fn into_template(self, _cx: &'a ScopeState) -> VNode<'a>;
}

impl<'a> IntoTemplate<'a> for VNode<'a> {
    fn into_template(self, _cx: &'a ScopeState) -> VNode<'a> {
        self
    }
}

impl<'a> IntoTemplate<'a> for Element<'a> {
    fn into_template(self, _cx: &'a ScopeState) -> VNode<'a> {
        match self {
            Some(val) => val.into_template(_cx),
            _ => VNode::empty().unwrap(),
        }
    }
}
impl<'a, 'b> IntoTemplate<'a> for LazyNodes<'a, 'b> {
    fn into_template(self, cx: &'a ScopeState) -> VNode<'a> {
        self.call(cx)
    }
}

// Note that we're using the E as a generic but this is never crafted anyways.
pub struct FromNodeIterator;
impl<'a, T, I> IntoDynNode<'a, FromNodeIterator> for T
where
    T: Iterator<Item = I>,
    I: IntoTemplate<'a>,
{
    fn into_vnode(self, cx: &'a ScopeState) -> DynamicNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(cx.bump());

        nodes.extend(self.into_iter().map(|node| node.into_template(cx)));

        match nodes.into_bump_slice() {
            children if children.is_empty() => DynamicNode::default(),
            children => DynamicNode::Fragment(children),
        }
    }
}
