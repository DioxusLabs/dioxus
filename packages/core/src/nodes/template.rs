use std::{cell::Cell, hash::Hash};

use crate::{Attribute, ElementId, Listener, VNode};

/// A reference to a template along with any context needed to hydrate it
pub struct VTemplate<'a> {
    pub key: Option<&'a str>,

    // The ID assigned for all nodes in this template
    pub node_id: Cell<ElementId>,

    // Position this template for fragments and stuff
    pub head_id: Cell<ElementId>,

    pub tail_id: Cell<ElementId>,

    pub template: Template<'static>,

    /// All the non-root dynamic nodes
    pub dynamic_nodes: &'a [NodeLocation<'a>],

    pub dynamic_attrs: &'a [AttributeLocation<'a>],

    pub listeners: &'a [Listener<'a>],
}

/// A template that is created at compile time
#[derive(Clone, Copy)]
pub struct Template<'a> {
    /// name, line, col, or some sort of identifier
    pub id: &'static str,

    /// All the roots of the template. ie rsx! { div {} div{} } would have two roots
    pub roots: &'a [TemplateNode<'a>],
}

impl<'a> Eq for Template<'a> {}
impl<'a> PartialEq for Template<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<'a> Hash for Template<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// A weird-ish variant of VNodes with way more limited types
pub enum TemplateNode<'a> {
    /// A simple element
    Element {
        tag: &'static str,
        namespace: Option<&'static str>,
        attrs: &'a [TemplateAttribute<'a>],
        children: &'a [TemplateNode<'a>],
    },
    Text(&'static str),
    Dynamic(usize),
}

pub struct TemplateAttribute<'a> {
    pub name: &'static str,
    pub value: &'a str,
    pub namespace: Option<&'static str>,
    pub volatile: bool,
}

pub struct AttributeLocation<'a> {
    pub pathway: &'static [u8],
    pub mounted_element: Cell<ElementId>,
    pub attrs: &'a [Attribute<'a>],
    pub listeners: &'a [Listener<'a>],
}

pub struct NodeLocation<'a> {
    pub pathway: &'static [u8],
    pub node: VNode<'a>,
}
