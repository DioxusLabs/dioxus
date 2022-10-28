use std::{cell::Cell, num::NonZeroUsize};

/// A reference to a template along with any context needed to hydrate it
pub struct VTemplate<'a> {
    // The ID assigned for the root of this template
    pub node_id: Cell<ElementId>,

    pub template: &'static Template,

    /// All the dynamic nodes for a template
    pub dynamic_nodes: &'a [DynamicNode<'a>],

    pub dynamic_attrs: &'a [AttributeLocation<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct Template {
    pub id: &'static str,

    pub root: TemplateNode<'static>,

    // todo: locations of dynamic nodes
    pub node_pathways: &'static [&'static [u8]],

    // todo: locations of dynamic nodes
    pub attr_pathways: &'static [&'static [u8]],
}

/// A weird-ish variant of VNodes with way more limited types
#[derive(Debug, Clone, Copy)]
pub enum TemplateNode<'a> {
    /// A simple element
    Element {
        tag: &'a str,
        namespace: Option<&'a str>,
        attrs: &'a [TemplateAttribute<'a>],
        children: &'a [TemplateNode<'a>],
    },
    Text(&'a str),
    Dynamic(usize),
    DynamicText(usize),
}

pub enum DynamicNode<'a> {
    // Anything declared in component form
    // IE in caps or with underscores
    Component {
        name: &'static str,
    },

    // Comes in with string interpolation or from format_args, include_str, etc
    Text {
        id: Cell<ElementId>,
        value: &'static str,
    },

    // Anything that's coming in as an iterator
    Fragment {
        children: &'a [VTemplate<'a>],
    },
}

#[derive(Debug)]
pub struct TemplateAttribute<'a> {
    pub name: &'static str,
    pub value: &'a str,
    pub namespace: Option<&'static str>,
    pub volatile: bool,
}

pub struct AttributeLocation<'a> {
    pub mounted_element: Cell<ElementId>,
    pub attrs: &'a [Attribute<'a>],
}

#[derive(Debug)]
pub struct Attribute<'a> {
    pub name: &'static str,
    pub value: &'a str,
    pub namespace: Option<&'static str>,
}

#[test]
fn what_are_the_sizes() {
    dbg!(std::mem::size_of::<VTemplate>());
    dbg!(std::mem::size_of::<Template>());
    dbg!(std::mem::size_of::<TemplateNode>());
}
