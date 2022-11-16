use std::{
    cell::{Cell, RefCell},
    fmt::Arguments,
};

use bumpalo::boxed::Box as BumpBox;
use bumpalo::Bump;
use std::future::Future;

use crate::{
    any_props::{AnyProps, VComponentProps},
    arena::ElementId,
    innerlude::{DynamicNode, EventHandler},
    Attribute, AttributeValue, Element, LazyNodes, Properties, Scope, ScopeState, VNode,
};

impl ScopeState {
    /// Create some text that's allocated along with the other vnodes
    pub fn text<'a>(&'a self, args: Arguments) -> DynamicNode<'a> {
        let (text, _) = self.raw_text(args);
        DynamicNode::Text {
            id: Cell::new(ElementId(0)),
            value: text,
            inner: false,
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

    pub fn fragment_from_iter<'a, 'c, I, J>(
        &'a self,
        node_iter: impl IntoVNode<'a, I, J> + 'c,
    ) -> DynamicNode {
        node_iter.into_vnode(self)

        // let mut bump_vec = bumpalo::vec![in self.bump();];

        // for item in it {
        //     bump_vec.push(item.into_vnode(self));
        // }

        // match bump_vec.len() {
        //     0 => DynamicNode::Placeholder(Cell::new(ElementId(0))),
        //     _ => DynamicNode::Fragment {
        //         inner: false,
        //         nodes: bump_vec.into_bump_slice(),
        //     },
        // }
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
            static_props: P::IS_STATIC,
            props: Cell::new(detached_dyn),
            placeholder: Cell::new(None),
            scope: Cell::new(None),
        }
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
    Sync(Element<'a>),
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

pub trait IntoVNode<'a, A = (), J = ()> {
    fn into_vnode(self, cx: &'a ScopeState) -> DynamicNode<'a>;
}

impl<'a, 'b> IntoVNode<'a> for VNode<'a> {
    fn into_vnode(self, _cx: &'a ScopeState) -> DynamicNode<'a> {
        todo!()
        // self
    }
}

impl<'a, 'b> IntoVNode<'a> for LazyNodes<'a, 'b> {
    fn into_vnode(self, cx: &'a ScopeState) -> DynamicNode<'a> {
        todo!()
        // self.call(cx)
    }
}

impl<'b> IntoVNode<'_> for &'b str {
    fn into_vnode(self, cx: &ScopeState) -> DynamicNode {
        // cx.text(format_args!("{}", self))
        todo!()
    }
}

impl IntoVNode<'_> for String {
    fn into_vnode(self, cx: &ScopeState) -> DynamicNode {
        // cx.text(format_args!("{}", self))
        todo!()
    }
}

impl<'b> IntoVNode<'b> for Arguments<'_> {
    fn into_vnode(self, cx: &'b ScopeState) -> DynamicNode<'b> {
        // cx.text(self)
        todo!()
    }
}

impl<'a, 'b> IntoVNode<'a> for &VNode<'a> {
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

// Note that we're using the E as a generic but this is never crafted anyways.
pub struct FromNodeIterator;
impl<'a, T, I, E> IntoVNode<'a, FromNodeIterator, E> for T
where
    T: IntoIterator<Item = I>,
    I: IntoVNode<'a, E>,
{
    fn into_vnode(self, cx: &'a ScopeState) -> DynamicNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(cx.bump());

        for node in self {
            nodes.push(node.into_vnode(cx));
        }

        let children = nodes.into_bump_slice();

        // if cfg!(debug_assertions) && children.len() > 1 && children.last().unwrap().key().is_none()
        // {
        // let bt = backtrace::Backtrace::new();
        // let bt = "no backtrace available";

        // // todo: make the backtrace prettier or remove it altogether
        // log::error!(
        //     r#"
        //     Warning: Each child in an array or iterator should have a unique "key" prop.
        //     Not providing a key will lead to poor performance with lists.
        //     See docs.rs/dioxus for more information.
        //     -------------
        //     {:?}
        //     "#,
        //     bt
        // );
        // }

        todo!()
        // VNode::Fragment(cx.bump.alloc(VFragment {
        //     children,
        //     placeholder: Default::default(),
        //     key: None,
        // }))
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
