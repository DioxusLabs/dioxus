use std::hash::Hash;

use crate::{Attribute, Listener, VNode};

/// A reference to a template along with any context needed to hydrate it
pub struct VTemplate<'a> {
    pub key: Option<&'a str>,

    pub template: Template<'static>,

    pub dynamic_nodes: &'a [VNode<'a>],

    pub dynamic_attrs: &'a [Attribute<'a>],

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
    Element {
        tag: &'static str,
        attrs: &'a [TemplateAttribute<'a>],
        children: &'a [TemplateNode<'a>],
    },
    Text(&'static str),
    Dynamic(usize),
}

pub enum TemplateAttribute<'a> {
    // todo: more values
    Static { name: &'static str, value: &'a str },
    Dynamic(usize),
}
