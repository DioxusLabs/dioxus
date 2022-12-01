use crate::{innerlude::DynamicNode, AttributeValue, Element, LazyNodes, ScopeState, VNode};
use bumpalo::boxed::Box as BumpBox;
use bumpalo::Bump;
use std::fmt::Arguments;
use std::future::Future;

#[doc(hidden)]
pub trait ComponentReturn<'a, A = ()> {
    fn into_return(self, cx: &'a ScopeState) -> RenderReturn<'a>;
}

impl<'a> ComponentReturn<'a> for Element<'a> {
    fn into_return(self, _cx: &ScopeState) -> RenderReturn<'a> {
        RenderReturn::Sync(self)
    }
}

#[doc(hidden)]
pub struct AsyncMarker;
impl<'a, F> ComponentReturn<'a, AsyncMarker> for F
where
    F: Future<Output = Element<'a>> + 'a,
{
    fn into_return(self, cx: &'a ScopeState) -> RenderReturn<'a> {
        let f: &mut dyn Future<Output = Element<'a>> = cx.bump().alloc(self);
        RenderReturn::Async(unsafe { BumpBox::from_raw(f) })
    }
}

pub enum RenderReturn<'a> {
    /// A currently-available element
    Sync(Element<'a>),

    /// An ongoing future that will resolve to a [`Element`]
    Async(BumpBox<'a, dyn Future<Output = Element<'a>> + 'a>),
}

impl<'a> RenderReturn<'a> {
    pub(crate) unsafe fn extend_lifetime_ref<'c>(&self) -> &'c RenderReturn<'c> {
        unsafe { std::mem::transmute(self) }
    }
    pub(crate) unsafe fn extend_lifetime<'c>(self) -> RenderReturn<'c> {
        unsafe { std::mem::transmute(self) }
    }
}

#[doc(hidden)]
pub trait IntoDynNode<'a, A = ()> {
    fn into_vnode(self, cx: &'a ScopeState) -> DynamicNode<'a>;
}

impl<'a> IntoDynNode<'a> for () {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        DynamicNode::placeholder()
    }
}
impl<'a> IntoDynNode<'a> for VNode<'a> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        DynamicNode::Fragment(_cx.bump().alloc([self]))
    }
}
impl<'a> IntoDynNode<'a> for Element<'a> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        match self {
            Ok(val) => val.into_vnode(_cx),
            _ => DynamicNode::placeholder(),
        }
    }
}

impl<'a, T: IntoDynNode<'a>> IntoDynNode<'a> for Option<T> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        match self {
            Some(val) => val.into_vnode(_cx),
            None => DynamicNode::placeholder(),
        }
    }
}

impl<'a> IntoDynNode<'a> for &Element<'a> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        match self.as_ref() {
            Ok(val) => val.clone().into_vnode(_cx),
            _ => DynamicNode::placeholder(),
        }
    }
}

impl<'a, 'b> IntoDynNode<'a> for LazyNodes<'a, 'b> {
    fn into_vnode(self, cx: &'a ScopeState) -> DynamicNode<'a> {
        DynamicNode::Fragment(cx.bump().alloc([self.call(cx)]))
    }
}

impl<'a> IntoDynNode<'_> for &'a str {
    fn into_vnode(self, cx: &ScopeState) -> DynamicNode {
        cx.text_node(format_args!("{}", self))
    }
}

impl IntoDynNode<'_> for String {
    fn into_vnode(self, cx: &ScopeState) -> DynamicNode {
        cx.text_node(format_args!("{}", self))
    }
}

impl<'b> IntoDynNode<'b> for Arguments<'_> {
    fn into_vnode(self, cx: &'b ScopeState) -> DynamicNode<'b> {
        cx.text_node(self)
    }
}

impl<'a> IntoDynNode<'a> for &'a VNode<'a> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        DynamicNode::Fragment(_cx.bump().alloc([VNode {
            parent: self.parent,
            template: self.template,
            root_ids: self.root_ids,
            key: self.key,
            dynamic_nodes: self.dynamic_nodes,
            dynamic_attrs: self.dynamic_attrs,
        }]))
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
impl<'a, 'b> IntoTemplate<'a> for LazyNodes<'a, 'b> {
    fn into_template(self, cx: &'a ScopeState) -> VNode<'a> {
        self.call(cx)
    }
}

// Note that we're using the E as a generic but this is never crafted anyways.
#[doc(hidden)]
pub struct FromNodeIterator;
impl<'a, T, I> IntoDynNode<'a, FromNodeIterator> for T
where
    T: Iterator<Item = I>,
    I: IntoTemplate<'a>,
{
    fn into_vnode(self, cx: &'a ScopeState) -> DynamicNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(cx.bump());

        for node in self {
            nodes.push(node.into_template(cx));
        }

        let children = nodes.into_bump_slice();

        match children.len() {
            0 => DynamicNode::placeholder(),
            _ => DynamicNode::Fragment(children),
        }
    }
}

/// A value that can be converted into an attribute value
pub trait IntoAttributeValue<'a> {
    /// Convert into an attribute value
    fn into_value(self, bump: &'a Bump) -> AttributeValue<'a>;
}

impl<'a> IntoAttributeValue<'a> for &'a str {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Text(self)
    }
}
impl<'a> IntoAttributeValue<'a> for f32 {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Float(self)
    }
}
impl<'a> IntoAttributeValue<'a> for i32 {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Int(self)
    }
}
impl<'a> IntoAttributeValue<'a> for bool {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Bool(self)
    }
}
impl<'a> IntoAttributeValue<'a> for Arguments<'_> {
    fn into_value(self, bump: &'a Bump) -> AttributeValue<'a> {
        use bumpalo::core_alloc::fmt::Write;
        let mut str_buf = bumpalo::collections::String::new_in(bump);
        str_buf.write_fmt(self).unwrap();
        AttributeValue::Text(str_buf.into_bump_str())
    }
}
