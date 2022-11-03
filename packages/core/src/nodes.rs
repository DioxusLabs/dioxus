use crate::{any_props::AnyProps, arena::ElementId};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    hash::Hasher,
};

pub type TemplateId = &'static str;

/// A reference to a template along with any context needed to hydrate it
pub struct VNode<'a> {
    // The ID assigned for the root of this template
    pub node_id: Cell<ElementId>,

    pub key: Option<&'a str>,

    // When rendered, this template will be linked to its parent manually
    pub parent: Option<(*mut VNode<'static>, usize)>,

    pub template: Template<'static>,

    pub root_ids: &'a [Cell<ElementId>],

    pub dynamic_nodes: &'a [DynamicNode<'a>],

    pub dynamic_attrs: &'a [Attribute<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct Template<'a> {
    pub id: &'a str,
    pub roots: &'a [TemplateNode<'a>],
    pub node_paths: &'a [&'a [u8]],
    pub attr_paths: &'a [&'a [u8]],
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

pub enum DynamicNode<'a> {
    // Anything declared in component form
    // IE in caps or with underscores
    Component {
        name: &'static str,
        can_memoize: bool,
        props: *mut dyn AnyProps<'a>,
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
    Dynamic(usize),
}

pub struct Attribute<'a> {
    pub name: &'a str,
    pub value: AttributeValue<'a>,
    pub namespace: Option<&'static str>,
    pub mounted_element: Cell<ElementId>,
    pub volatile: bool,
}

pub enum AttributeValue<'a> {
    Text(&'a str),
    Float(f32),
    Int(i32),
    Bool(bool),
    Listener(RefCell<&'a mut dyn FnMut(&dyn Any)>),
    Any(&'a dyn AnyValue),
    None,
}

impl<'a> AttributeValue<'a> {
    fn is_listener(&self) -> bool {
        matches!(self, AttributeValue::Listener(_))
    }
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

#[test]
fn what_are_the_sizes() {
    dbg!(std::mem::size_of::<VNode>());
    dbg!(std::mem::size_of::<Template>());
    dbg!(std::mem::size_of::<TemplateNode>());
}
