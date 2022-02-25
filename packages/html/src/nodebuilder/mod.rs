pub use crate::builder::IntoAttributeValue;
use bumpalo::collections::Vec as BumpVec;
use dioxus_core::{
    self, exports::bumpalo, Attribute, Element, IntoVNode, Listener, NodeFactory, ScopeState,
    VNode, VText,
};

pub mod events;
pub mod fragments;

pub struct ElementBuilder<'a, T> {
    _inner: T, // a zst marker type
    name: &'static str,
    fac: NodeFactory<'a>,
    attrs: BumpVec<'a, Attribute<'a>>,
    children: BumpVec<'a, VNode<'a>>,
    listeners: BumpVec<'a, Listener<'a>>,
    namespace: Option<&'static str>,
}

impl<'a, T> ElementBuilder<'a, T> {
    pub fn new(cx: &'a ScopeState, t: T, name: &'static str) -> Self {
        let fac = NodeFactory::new(cx);
        ElementBuilder {
            attrs: BumpVec::new_in(fac.bump()),
            children: BumpVec::new_in(fac.bump()),
            listeners: BumpVec::new_in(fac.bump()),
            _inner: t,
            name,
            fac,
            namespace: None,
        }
    }

    pub fn new_svg(cx: &'a ScopeState, t: T, name: &'static str) -> Self {
        let fac = NodeFactory::new(cx);
        ElementBuilder {
            attrs: BumpVec::new_in(fac.bump()),
            children: BumpVec::new_in(fac.bump()),
            listeners: BumpVec::new_in(fac.bump()),
            _inner: t,
            name,
            fac,
            namespace: Some("http://www.w3.org/2000/svg"),
        }
    }

    pub fn push_attr(&mut self, name: &'static str, val: impl IntoAttributeValue<'a>) {
        let (value, is_static) = val.into_str(self.fac);
        self.attrs.push(Attribute {
            name,
            value,
            is_static,
            namespace: None,
            is_volatile: false,
        });
    }
}

impl<'a, T> IntoVNode<'a> for ElementBuilder<'a, T> {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        todo!()
    }
}

fn text<'a>(cx: &'a ScopeState, val: impl IntoAttributeValue<'a>) -> VNode<'a> {
    let fac = NodeFactory::new(cx);
    let (text, is_static) = val.into_str(fac);
    VNode::Text(fac.bump().alloc(VText {
        text,
        is_static,
        id: Default::default(),
    }))
}

macro_rules! no_namespace_trait_methods {
    (
        $(
            $(#[$attr:meta])*
            $name:ident;
        )*
    ) => {
        $(
            $(#[$attr])*
            pub fn $name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
                let (value, is_static) = val.into_str(self.fac);
                self.attrs.push(Attribute {
                    name: stringify!($name),
                    value,
                    is_static,
                    namespace: None,
                    is_volatile: false,
                });
                self
            }
        )*
    };
}

impl<'a, T> ElementBuilder<'a, T> {
    /// Build this node builder into a VNode.
    pub fn render(self) -> Option<VNode<'a>> {
        Some(self.fac.raw_element(
            self.name,
            None,
            self.listeners.into_bump_slice(),
            self.attrs.into_bump_slice(),
            self.children.into_bump_slice(),
            None,
        ))
    }

    pub fn hints(mut self, listeners: usize, attrs: usize, children: usize) -> Self {
        self.listeners.reserve(listeners);
        self.attrs.reserve(attrs);
        self.children.reserve(children);
        self
    }

    pub fn attr(mut self, name: &'static str, val: impl IntoAttributeValue<'a>) -> Self {
        let (value, is_static) = val.into_str(self.fac);
        self.attrs.push(Attribute {
            name,
            value,
            is_static,
            namespace: None,
            is_volatile: false,
        });
        self
    }

    pub fn style_attr(mut self, name: &'static str, val: impl IntoAttributeValue<'a>) -> Self {
        let (value, is_static) = val.into_str(self.fac);
        self.attrs.push(Attribute {
            name,
            value,
            is_static,
            namespace: Some("style"),
            is_volatile: false,
        });
        self
    }

    /// Add a bunch of pre-formatted attributes
    pub fn attributes(mut self, iter: impl IntoIterator<Item = Attribute<'a>>) -> Self {
        for attr in iter {
            self.attrs.push(attr);
        }
        self
    }

    pub fn attr_ns(
        mut self,
        name: &'static str,
        name_space: &'static str,
        val: impl IntoAttributeValue<'a>,
    ) -> Self {
        let (value, is_static) = val.into_str(self.fac);
        self.attrs.push(Attribute {
            name,
            value,
            is_static,
            namespace: Some(name_space),
            is_volatile: false,
        });
        self
    }

    pub fn children<'b, 'c, F, A>(mut self, node_iter: A) -> Self
    where
        F: IntoVNode<'a>,

        // two trait requirements but we use one
        // this forces all pure iterators to come in as fragments
        A: AsRef<[F]> + IntoIterator<Item = F>,
    {
        for node in node_iter {
            self.children.push(node.into_vnode(self.fac));
        }
        self
    }

    /// Add a child fragment from an iterator
    pub fn fragment<'b, 'c>(
        mut self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a> + 'c> + 'b,
    ) -> Self {
        self.children.push(self.fac.fragment_from_iter(node_iter));
        self
    }

    no_namespace_trait_methods! {
        /// accesskey
        accesskey;

        /// class
        class;

        /// contenteditable
        contenteditable;

        /// data
        data;

        /// dir
        dir;

        /// draggable
        draggable;

        /// hidden
        hidden;

        /// Set the value of the `id` attribute.
        id;

        /// lang
        lang;

        /// spellcheck
        spellcheck;

        /// style
        style;

        /// tabindex
        tabindex;

        /// title
        title;

        /// translate
        translate;

        /// role
        role;

        /// dangerous_inner_html
        dangerous_inner_html;
    }
}

#[test]
fn test_builder() {
    use crate::codegen::elements::*;
    use dioxus_core::prelude::*;

    fn please(cx: Scope) -> Element {
        div(&cx)
            .class("a")
            .draggable(false)
            .id("asd")
            .accesskey(false)
            .class(false)
            .contenteditable(false)
            .data(false)
            .dir(false)
            .dangerous_inner_html(false)
            .attr("name", "asd")
            .onclick(move |_| println!("clicked"))
            .onclick(move |_| println!("clicked"))
            .onclick(move |_| println!("clicked"))
            .children([
                match true {
                    true => div(&cx),
                    false => div(&cx).class("asd"),
                },
                match 10 {
                    10 => div(&cx),
                    _ => div(&cx).class("asd"),
                },
            ])
            .children([
                match true {
                    true => div(&cx),
                    false => div(&cx).class("asd"),
                },
                match 10 {
                    10 => div(&cx),
                    _ => div(&cx).class("asd"),
                },
            ])
            .fragment((0..10).map(|i| {
                div(&cx)
                    .class("val")
                    .class(format_args!("{}", i))
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
            }))
            .fragment((0..10).map(|i| div(&cx).class("val")))
            .fragment((0..20).map(|i| div(&cx).class("val")))
            .fragment((0..30).map(|i| div(&cx).class("val")))
            .fragment((0..40).map(|i| div(&cx).class("val")))
            .children([
                match true {
                    true => div(&cx),
                    false => div(&cx).class("asd"),
                },
                match 10 {
                    10 => div(&cx),
                    _ => div(&cx).class("asd"),
                },
                if 20 == 10 {
                    div(&cx)
                } else {
                    div(&cx).class("asd")
                },
            ])
            .render()
    }
}
