use crate::{any_props::AnyProps, arena::ElementId};
use std::{any::Any, cell::Cell, hash::Hasher};

pub type TemplateId = &'static str;

/// A reference to a template along with any context needed to hydrate it
pub struct VNode<'a> {
    // The ID assigned for the root of this template
    pub node_id: Cell<ElementId>,

    // When rendered, this template will be linked to its parent
    pub parent: Option<(*mut VNode<'static>, usize)>,

    pub template: Template<'static>,

    pub root_ids: &'a [Cell<ElementId>],

    /// All the dynamic nodes for a template
    pub dynamic_nodes: &'a mut [DynamicNode<'a>],

    pub dynamic_attrs: &'a mut [AttributeLocation<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct Template<'a> {
    pub id: &'a str,
    pub roots: &'a [TemplateNode<'a>],
}

impl<'a> std::hash::Hash for Template<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Template<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Template<'_> {}
impl PartialOrd for Template<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(other.id)
    }
}
impl Ord for Template<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(other.id)
    }
}

/// A weird-ish variant of VNodes with way more limited types
#[derive(Debug, Clone, Copy)]
pub enum TemplateNode<'a> {
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

pub struct DynamicNode<'a> {
    pub path: &'static [u8],
    pub kind: DynamicNodeKind<'a>,
}

pub enum DynamicNodeKind<'a> {
    // Anything declared in component form
    // IE in caps or with underscores
    Component {
        name: &'static str,
        props: *mut dyn AnyProps,
    },

    // Comes in with string interpolation or from format_args, include_str, etc
    Text {
        id: Cell<ElementId>,
        value: &'a str,
    },

    // Anything that's coming in as an iterator
    Fragment {
        children: &'a [VNode<'a>],
    },
}

#[derive(Debug)]
pub enum TemplateAttribute<'a> {
    Static {
        name: &'static str,
        value: &'a str,
        namespace: Option<&'static str>,
        volatile: bool,
    },
    Dynamic {
        name: &'static str,
        index: usize,
    },
}

pub struct AttributeLocation<'a> {
    pub mounted_element: Cell<ElementId>,
    pub attrs: &'a mut [Attribute<'a>],
    pub listeners: &'a mut [Listener<'a>],
    pub path: &'static [u8],
}

#[derive(Debug)]
pub struct Attribute<'a> {
    pub name: &'static str,
    pub value: &'a str,
    pub namespace: Option<&'static str>,
}

pub enum AttributeValue<'a> {
    Text(&'a str),
    Float(f32),
    Int(i32),
    Bool(bool),
    Any(&'a dyn AnyValue),
}

pub trait AnyValue {
    fn any_cmp(&self, other: &dyn Any) -> bool;
}
impl<T> AnyValue for T
where
    T: PartialEq + Any,
{
    fn any_cmp(&self, other: &dyn Any) -> bool {
        if self.type_id() != other.type_id() {
            return false;
        }

        self == unsafe { &*(other as *const _ as *const T) }
    }
}

pub struct Listener<'a> {
    pub name: &'static str,
    pub callback: &'a mut dyn FnMut(&dyn Any),
}

#[test]
fn what_are_the_sizes() {
    dbg!(std::mem::size_of::<VNode>());
    dbg!(std::mem::size_of::<Template>());
    dbg!(std::mem::size_of::<TemplateNode>());
}
