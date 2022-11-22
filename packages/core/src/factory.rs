use std::{
    cell::{Cell, RefCell},
    fmt::Arguments,
};

use bumpalo::boxed::Box as BumpBox;
use bumpalo::Bump;
use std::future::Future;

use crate::{
    any_props::{AnyProps, VProps},
    arena::ElementId,
    innerlude::{DynamicNode, EventHandler, VComponent, VFragment, VText},
    Attribute, AttributeValue, Element, LazyNodes, Properties, Scope, ScopeState, VNode,
};

impl ScopeState {
    /// Create some text that's allocated along with the other vnodes
    pub fn text<'a>(&'a self, args: Arguments) -> DynamicNode<'a> {
        let (text, _) = self.raw_text(args);
        DynamicNode::Text(VText {
            id: Cell::new(ElementId(0)),
            value: text,
        })
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

    pub fn fragment_from_iter<'a, 'c, I>(
        &'a self,
        node_iter: impl IntoDynNode<'a, I> + 'c,
    ) -> DynamicNode {
        node_iter.into_vnode(self)
    }

    /// Create a new [`Attribute`]
    pub fn attr<'a>(
        &'a self,
        name: &'static str,
        value: impl IntoAttributeValue<'a>,
        namespace: Option<&'static str>,
        volatile: bool,
    ) -> Attribute<'a> {
        Attribute {
            name,
            namespace,
            volatile,
            value: value.into_value(self.bump()),
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
        let as_component = component;
        let vcomp = VProps::new(as_component, P::memoize, props);
        let as_dyn: Box<dyn AnyProps<'a>> = Box::new(vcomp);
        let extended: Box<dyn AnyProps> = unsafe { std::mem::transmute(as_dyn) };

        // let as_dyn: &dyn AnyProps = self.bump().alloc(vcomp);
        // todo: clean up borrowed props
        // if !P::IS_STATIC {
        //     let vcomp = &*vcomp;
        //     let vcomp = unsafe { std::mem::transmute(vcomp) };
        //     self.scope.items.borrow_mut().borrowed_props.push(vcomp);
        // }

        DynamicNode::Component(VComponent {
            name: fn_name,
            render_fn: component as *const (),
            static_props: P::IS_STATIC,
            props: Cell::new(Some(extended)),
            placeholder: Cell::new(None),
            scope: Cell::new(None),
        })
    }

    /// Create a new [`EventHandler`] from an [`FnMut`]
    pub fn event_handler<'a, T>(&'a self, f: impl FnMut(T) + 'a) -> EventHandler<'a, T> {
        let handler: &mut dyn FnMut(T) = self.bump().alloc(f);
        let caller = unsafe { BumpBox::from_raw(handler as *mut dyn FnMut(T)) };
        let callback = RefCell::new(Some(caller));
        EventHandler { callback }
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
        let pined: BumpBox<_> = boxed.into();
        RenderReturn::Async(pined)
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

pub trait IntoDynNode<'a, A = ()> {
    fn into_vnode(self, cx: &'a ScopeState) -> DynamicNode<'a>;
}

impl<'a, 'b> IntoDynNode<'a> for () {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        todo!()
        // self
    }
}
impl<'a, 'b> IntoDynNode<'a> for VNode<'a> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        // DynamicNode::Fragment { nodes: cx., inner: () }
        todo!()
    }
}

impl<'a, 'b, T: IntoDynNode<'a>> IntoDynNode<'a> for Option<T> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        match self {
            Some(val) => val.into_vnode(_cx),
            None => DynamicNode::Placeholder(Default::default()),
        }
    }
}

impl<'a, 'b, T: IntoDynNode<'a>> IntoDynNode<'a> for &Option<T> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        // DynamicNode::Fragment { nodes: cx., inner: () }
        todo!()
    }
}

impl<'a, 'b> IntoDynNode<'a> for LazyNodes<'a, 'b> {
    fn into_vnode(self, cx: &'a ScopeState) -> DynamicNode<'a> {
        DynamicNode::Fragment(VFragment {
            nodes: cx.bump().alloc([self.call(cx)]),
        })
    }
}

impl<'b> IntoDynNode<'_> for &'b str {
    fn into_vnode(self, cx: &ScopeState) -> DynamicNode {
        cx.text(format_args!("{}", self))
    }
}

impl IntoDynNode<'_> for String {
    fn into_vnode(self, cx: &ScopeState) -> DynamicNode {
        cx.text(format_args!("{}", self))
    }
}

impl<'b> IntoDynNode<'b> for Arguments<'_> {
    fn into_vnode(self, cx: &'b ScopeState) -> DynamicNode<'b> {
        cx.text(self)
    }
}

impl<'a, 'b> IntoDynNode<'a> for &VNode<'a> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        todo!()
        // VNode {
        //     node_id: self.node_id.clone(),
        //     parent: self.parent,
        //     template: self.template,
        //     root_ids: self.root_ids,
        //     key: self.key,
        //     dynamic_nodes: self.dynamic_nodes,
        //     dynamic_attrs: self.dynamic_attrs,
        // }
    }
}

pub trait IntoTemplate<'a> {
    fn into_template(self, _cx: &'a ScopeState) -> VNode<'a>;
}
impl<'a, 'b> IntoTemplate<'a> for VNode<'a> {
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
            0 => DynamicNode::Placeholder(Cell::new(ElementId(0))),
            _ => DynamicNode::Fragment(VFragment { nodes: children }),
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
