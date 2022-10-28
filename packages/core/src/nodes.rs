use crate::{any_props::AnyProps, arena::ElementId, scopes::ComponentPtr};
use std::{
    any::{Any, TypeId},
    cell::Cell,
    num::NonZeroUsize,
};

pub type TemplateId = &'static str;

/// A reference to a template along with any context needed to hydrate it
pub struct VTemplate<'a> {
    // The ID assigned for the root of this template
    pub node_id: Cell<ElementId>,

    pub template: &'static Template,

    pub root_ids: &'a [Cell<ElementId>],

    /// All the dynamic nodes for a template
    pub dynamic_nodes: &'a mut [DynamicNode<'a>],

    pub dynamic_attrs: &'a mut [AttributeLocation<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct Template {
    pub id: &'static str,

    pub roots: &'static [TemplateNode<'static>],
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

pub struct DynamicNode<'a> {
    pub path: &'static [u8],
    pub kind: DynamicNodeKind<'a>,
}

pub enum DynamicNodeKind<'a> {
    // Anything declared in component form
    // IE in caps or with underscores
    Component {
        name: &'static str,
        fn_ptr: ComponentPtr,
        props: Box<dyn AnyProps>,
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
    pub listeners: &'a [Listener<'a>],
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
    pub callback: &'a dyn Fn(),
}

#[test]
fn what_are_the_sizes() {
    dbg!(std::mem::size_of::<VTemplate>());
    dbg!(std::mem::size_of::<Template>());
    dbg!(std::mem::size_of::<TemplateNode>());
}
