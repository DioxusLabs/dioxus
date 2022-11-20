use crate::{any_props::AnyProps, arena::ElementId, ScopeId, ScopeState, UiEvent};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    hash::Hasher,
};

pub type TemplateId = &'static str;

/// A reference to a template along with any context needed to hydrate it
///
/// The dynamic parts of the template are stored separately from the static parts. This allows faster diffing by skipping
/// static parts of the template.
#[derive(Debug)]
pub struct VNode<'a> {
    // The ID assigned for the root of this template
    pub node_id: Cell<ElementId>,

    /// The key given to the root of this template.
    ///
    /// In fragments, this is the key of the first child. In other cases, it is the key of the root.
    pub key: Option<&'a str>,

    /// When rendered, this template will be linked to its parent manually
    pub parent: Option<ElementId>,

    /// The static nodes and static descriptor of the template
    pub template: Template<'static>,

    /// The IDs for the roots of this template - to be used when moving the template around and removing it from
    /// the actual Dom
    pub root_ids: &'a [Cell<ElementId>],

    /// The dynamic parts of the template
    pub dynamic_nodes: &'a [DynamicNode<'a>],

    /// The dynamic parts of the template
    pub dynamic_attrs: &'a [Attribute<'a>],
}

impl<'a> VNode<'a> {
    pub fn single_component(
        cx: &'a ScopeState,
        node: DynamicNode<'a>,
        id: &'static str,
    ) -> Option<Self> {
        Some(VNode {
            node_id: Cell::new(ElementId(0)),
            key: None,
            parent: None,
            root_ids: &[],
            dynamic_nodes: cx.bump().alloc([node]),
            dynamic_attrs: &[],
            template: Template {
                id,
                roots: &[TemplateNode::Dynamic(0)],
                node_paths: &[&[0]],
                attr_paths: &[],
            },
        })
    }

    pub fn single_text(
        _cx: &'a ScopeState,
        text: &'static [TemplateNode<'static>],
        id: &'static str,
    ) -> Option<Self> {
        Some(VNode {
            node_id: Cell::new(ElementId(0)),
            key: None,
            parent: None,
            root_ids: &[],
            dynamic_nodes: &[],
            dynamic_attrs: &[],
            template: Template {
                id,
                roots: text,
                node_paths: &[&[0]],
                attr_paths: &[],
            },
        })
    }
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
impl Eq for Template<'_> {}
impl PartialEq for Template<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
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
        inner_opt: bool,
    },
    Text(&'a str),
    Dynamic(usize),
    DynamicText(usize),
}

#[derive(Debug)]
pub enum DynamicNode<'a> {
    Component(VComponent<'a>),
    Text(VText<'a>),
    Fragment(VFragment<'a>),
    Placeholder(Cell<ElementId>),
}

impl<'a> DynamicNode<'a> {
    pub fn is_component(&self) -> bool {
        matches!(self, DynamicNode::Component(_))
    }
}

pub struct VComponent<'a> {
    pub name: &'static str,
    pub static_props: bool,
    pub placeholder: Cell<Option<ElementId>>,
    pub scope: Cell<Option<ScopeId>>,
    pub props: Cell<Option<Box<dyn AnyProps<'a> + 'a>>>,
    pub render_fn: *const (),
}

impl<'a> std::fmt::Debug for VComponent<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponent")
            .field("name", &self.name)
            .field("static_props", &self.static_props)
            .field("placeholder", &self.placeholder)
            .field("scope", &self.scope)
            .finish()
    }
}

#[derive(Debug)]
pub struct VText<'a> {
    pub id: Cell<ElementId>,
    pub value: &'a str,
}

#[derive(Debug)]
pub struct VFragment<'a> {
    pub nodes: &'a [VNode<'a>],
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

#[derive(Debug)]
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
    Listener(RefCell<&'a mut dyn FnMut(UiEvent<dyn Any>)>),
    Any(&'a dyn AnyValue),
    None,
}

impl<'a> AttributeValue<'a> {
    pub fn new_listener<T: 'static>(
        cx: &'a ScopeState,
        mut f: impl FnMut(UiEvent<T>) + 'a,
    ) -> AttributeValue<'a> {
        AttributeValue::Listener(RefCell::new(cx.bump().alloc(
            move |event: UiEvent<dyn Any>| {
                if let Ok(data) = event.data.downcast::<T>() {
                    f(UiEvent {
                        bubbles: event.bubbles,
                        data,
                    })
                }
            },
        )))
    }
}

impl<'a> std::fmt::Debug for AttributeValue<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(arg0) => f.debug_tuple("Text").field(arg0).finish(),
            Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
            Self::Int(arg0) => f.debug_tuple("Int").field(arg0).finish(),
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
            Self::Listener(_) => f.debug_tuple("Listener").finish(),
            Self::Any(_) => f.debug_tuple("Any").finish(),
            Self::None => write!(f, "None"),
        }
    }
}

impl<'a> PartialEq for AttributeValue<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(l0), Self::Text(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Listener(_), Self::Listener(_)) => true,
            (Self::Any(l0), Self::Any(r0)) => l0.any_cmp(*r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl<'a> AttributeValue<'a> {
    pub fn matches_type(&self, other: &'a AttributeValue<'a>) -> bool {
        match (self, other) {
            (Self::Text(_), Self::Text(_)) => true,
            (Self::Float(_), Self::Float(_)) => true,
            (Self::Int(_), Self::Int(_)) => true,
            (Self::Bool(_), Self::Bool(_)) => true,
            (Self::Listener(_), Self::Listener(_)) => true,
            (Self::Any(_), Self::Any(_)) => true,
            _ => return false,
        }
    }
}

pub trait AnyValue {
    fn any_cmp(&self, other: &dyn AnyValue) -> bool;
    fn our_typeid(&self) -> TypeId;
}

impl<T: PartialEq + Any> AnyValue for T {
    fn any_cmp(&self, other: &dyn AnyValue) -> bool {
        if self.type_id() != other.our_typeid() {
            return false;
        }

        self == unsafe { &*(other as *const _ as *const T) }
    }

    fn our_typeid(&self) -> TypeId {
        self.type_id()
    }
}

#[test]
fn what_are_the_sizes() {
    dbg!(std::mem::size_of::<VNode>());
    dbg!(std::mem::size_of::<Template>());
    dbg!(std::mem::size_of::<TemplateNode>());
}
