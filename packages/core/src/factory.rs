use std::{cell::Cell, fmt::Arguments, pin::Pin};

use bumpalo::boxed::Box as BumpBox;
use bumpalo::Bump;
use futures_util::Future;

use crate::{
    any_props::{AnyProps, VComponentProps},
    arena::ElementId,
    innerlude::DynamicNode,
    Attribute, AttributeValue, Element, LazyNodes, Properties, Scope, ScopeState, VNode,
};

impl ScopeState {
    /// Create some text that's allocated along with the other vnodes
    pub fn text<'a>(&'a self, args: Arguments) -> DynamicNode<'a> {
        let (text, _) = self.raw_text(args);
        DynamicNode::Text {
            id: Cell::new(ElementId(0)),
            value: text,
        }
    }

    pub fn raw_text_inline<'a>(&'a self, args: Arguments) -> &'a str {
        self.raw_text(args).0
    }

    pub fn raw_text<'a>(&'a self, args: Arguments) -> (&'a str, bool) {
        match args.as_str() {
            Some(static_str) => (static_str, true),
            None => {
                use bumpalo::core_alloc::fmt::Write;
                let mut str_buf = bumpalo::collections::String::new_in(self.bump());
                str_buf.write_fmt(args).unwrap();
                (str_buf.into_bump_str(), false)
            }
        }
    }

    pub fn fragment_from_iter<'a, I, F: IntoVnode<'a, I>>(
        &'a self,
        it: impl IntoIterator<Item = F>,
    ) -> DynamicNode {
        let mut bump_vec = bumpalo::vec![in self.bump();];

        for item in it {
            bump_vec.push(item.into_dynamic_node(self));
        }

        match bump_vec.len() {
            0 => DynamicNode::Placeholder(Cell::new(ElementId(0))),
            _ => DynamicNode::Fragment(bump_vec.into_bump_slice()),
        }
    }

    /// Create a new [`Attribute`]
    pub fn attr<'a>(
        &'a self,
        name: &'static str,
        val: impl IntoAttributeValue<'a>,
        namespace: Option<&'static str>,
        volatile: bool,
    ) -> Attribute<'a> {
        Attribute {
            name,
            namespace,
            volatile,
            value: val.into_value(self.bump()),
            mounted_element: Cell::new(ElementId(0)),
        }
    }

    /// Create a new [`VNode::Component`]
    pub fn component<'a, P, A, F: ComponentReturn<'a, A>>(
        &'a self,
        component: fn(Scope<'a, P>) -> F,
        props: P,
        fn_name: &'static str,
    ) -> DynamicNode<'a>
    where
        P: Properties + 'a,
    {
        let props = self.bump().alloc(props);
        let as_component = component;
        let vcomp = VComponentProps::new(as_component, P::memoize, props);
        let as_dyn = self.bump().alloc(vcomp) as &mut dyn AnyProps;
        let detached_dyn: *mut dyn AnyProps = unsafe { std::mem::transmute(as_dyn) };

        // todo: clean up borrowed props
        // if !P::IS_STATIC {
        //     let vcomp = &*vcomp;
        //     let vcomp = unsafe { std::mem::transmute(vcomp) };
        //     self.scope.items.borrow_mut().borrowed_props.push(vcomp);
        // }

        DynamicNode::Component {
            name: fn_name,
            is_static: P::IS_STATIC,
            props: Cell::new(detached_dyn),
        }
    }
}

pub trait ComponentReturn<'a, A = ()> {
    fn as_return(self, cx: &'a ScopeState) -> RenderReturn<'a>;
}
impl<'a> ComponentReturn<'a> for Element<'a> {
    fn as_return(self, _cx: &ScopeState) -> RenderReturn<'a> {
        RenderReturn::Sync(self)
    }
}

pub struct AsyncMarker;
impl<'a, F> ComponentReturn<'a, AsyncMarker> for F
where
    F: Future<Output = Element<'a>> + 'a,
{
    fn as_return(self, cx: &'a ScopeState) -> RenderReturn<'a> {
        let f: &mut dyn Future<Output = Element<'a>> = cx.bump().alloc(self);
        let boxed = unsafe { BumpBox::from_raw(f) };
        let pined: Pin<BumpBox<_>> = boxed.into();
        RenderReturn::Async(pined)
    }
}

#[test]
fn takes_it() {
    fn demo(cx: Scope) -> Element {
        todo!()
    }
}

pub enum RenderReturn<'a> {
    Sync(Element<'a>),
    Async(Pin<BumpBox<'a, dyn Future<Output = Element<'a>> + 'a>>),
}

pub trait IntoVnode<'a, A = ()> {
    fn into_dynamic_node(self, cx: &'a ScopeState) -> VNode<'a>;
}

impl<'a, 'b> IntoVnode<'a> for LazyNodes<'a, 'b> {
    fn into_dynamic_node(self, cx: &'a ScopeState) -> VNode<'a> {
        self.call(cx)
    }
}

impl<'a, 'b> IntoVnode<'a> for VNode<'a> {
    fn into_dynamic_node(self, _cx: &'a ScopeState) -> VNode<'a> {
        self
    }
}

impl<'a, 'b> IntoVnode<'a> for &'a VNode<'a> {
    fn into_dynamic_node(self, _cx: &'a ScopeState) -> VNode<'a> {
        VNode {
            node_id: self.node_id.clone(),
            parent: self.parent,
            template: self.template,
            root_ids: self.root_ids,
            key: self.key,
            dynamic_nodes: self.dynamic_nodes,
            dynamic_attrs: self.dynamic_attrs,
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
