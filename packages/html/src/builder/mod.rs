use std::fmt::Arguments;

use crate::into_attr::*;
use bumpalo::collections::Vec as BumpVec;
use dioxus_core::{
    self, exports::bumpalo, Attribute, Element, IntoVNode, Listener, NodeFactory, Scope,
    ScopeState, VNode, VText,
};
mod elements;
mod events;
mod fragments;
use fragments::fragment;

pub struct ElementBuilder<'a, T> {
    _inner: T, // a marker type
    tag_name: &'static str,
    fac: NodeFactory<'a>,
    attrs: BumpVec<'a, Attribute<'a>>,
    children: BumpVec<'a, VNode<'a>>,
    listeners: BumpVec<'a, Listener<'a>>,
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
    pub fn build(self) -> VNode<'a> {
        self.fac.raw_element(
            self.tag_name,
            None,
            self.listeners.into_bump_slice(),
            self.attrs.into_bump_slice(),
            self.children.into_bump_slice(),
            None,
        )
    }
    pub fn build_some(self) -> Element<'a> {
        Some(self.fac.raw_element(
            self.tag_name,
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

    pub fn children<'b, 'c>(mut self, node_iter: impl AsRef<[VNode<'a>]>) -> Self {
        // let frag = self.fac.fragment_from_iter(node_iter);

        self
    }

    pub fn fragment<'b, 'c>(
        mut self,
        frag: impl IntoIterator<Item = impl IntoVNode<'a> + 'c> + 'b,
    ) -> Self {
        self
    }

    no_namespace_trait_methods! {
        accesskey;
        class;
        contenteditable;
        data;
        dir;
        draggable;
        hidden;
        /// Set the value of the `id` attribute.
        id;
        lang;
        spellcheck;
        style;
        tabindex;
        title;
        translate;
        role;
        dangerous_inner_html;
    }
}

#[test]
fn test_builder() {
    use elements::*;

    fn please(cx: Scope) -> VNode {
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
                    true => div(&cx).build(),
                    false => div(&cx).class("asd").build(),
                },
                match 10 {
                    10 => div(&cx).build(),
                    _ => div(&cx).class("asd").build(),
                },
            ])
            .children([
                match true {
                    true => div(&cx).build(),
                    false => div(&cx).class("asd").build(),
                },
                match 10 {
                    10 => div(&cx).build(),
                    _ => div(&cx).class("asd").build(),
                },
            ])
            .fragment((0..10).map(|i| {
                div(&cx)
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
                    .class("val")
                    .build()
            }))
            .fragment((0..10).map(|i| div(&cx).class("val").class("val").class("val").build()))
            .fragment((0..20).map(|i| div(&cx).class("val").class("val").class("val").build()))
            .fragment((0..30).map(|i| div(&cx).class("val").class("val").class("val").build()))
            .fragment((0..40).map(|i| div(&cx).class("val").class("val").class("val").build()))
            .children([
                match true {
                    true => div(&cx).build(),
                    false => div(&cx).class("asd").build(),
                },
                match 10 {
                    10 => div(&cx).build(),
                    _ => div(&cx).class("asd").build(),
                },
            ])
            .build()
    }

    fn test2(cx: Scope) -> Element {
        let count = &*cx.use_hook(|_| 0);

        cx.fragment([
            cx.div()
                .onclick(move |_| println!("{count}"))
                .inner_text("up high!")
                .build(),
            cx.div()
                .onclick(move |_| println!("{count}"))
                .inner_text("down low!")
                .build(),
        ])
    }
}
