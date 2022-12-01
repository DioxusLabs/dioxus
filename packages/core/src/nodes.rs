use crate::{any_props::AnyProps, arena::ElementId, Element, Event, ScopeId, ScopeState};
use bumpalo::boxed::Box as BumpBox;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
};

pub type TemplateId = &'static str;

/// A reference to a template along with any context needed to hydrate it
///
/// The dynamic parts of the template are stored separately from the static parts. This allows faster diffing by skipping
/// static parts of the template.
#[derive(Debug, Clone)]
pub struct VNode<'a> {
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
    pub fn empty() -> Element<'a> {
        Ok(VNode {
            key: None,
            parent: None,
            root_ids: &[],
            dynamic_nodes: &[],
            dynamic_attrs: &[],
            template: Template {
                id: "dioxus-empty",
                roots: &[],
                node_paths: &[],
                attr_paths: &[],
            },
        })
    }

    pub fn dynamic_root(&self, idx: usize) -> Option<&'a DynamicNode<'a>> {
        match &self.template.roots[idx] {
            TemplateNode::Element { .. } | TemplateNode::Text(_) => None,
            TemplateNode::Dynamic(id) | TemplateNode::DynamicText(id) => {
                Some(&self.dynamic_nodes[*id])
            }
        }
    }
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[derive(Debug, Clone, Copy)]
pub struct Template<'a> {
    pub id: &'a str,
    pub roots: &'a [TemplateNode<'a>],
    pub node_paths: &'a [&'a [u8]],
    pub attr_paths: &'a [&'a [u8]],
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
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
    Placeholder(Cell<ElementId>),
    Fragment(&'a [VNode<'a>]),
}

impl<'a> DynamicNode<'a> {
    pub fn is_component(&self) -> bool {
        matches!(self, DynamicNode::Component(_))
    }
    pub fn placeholder() -> Self {
        Self::Placeholder(Default::default())
    }
}

pub struct VComponent<'a> {
    pub name: &'static str,
    pub static_props: bool,
    pub scope: Cell<Option<ScopeId>>,
    pub render_fn: *const (),
    pub(crate) props: Cell<Option<Box<dyn AnyProps<'a> + 'a>>>,
}

impl<'a> std::fmt::Debug for VComponent<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponent")
            .field("name", &self.name)
            .field("static_props", &self.static_props)
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
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum TemplateAttribute<'a> {
    Static {
        name: &'a str,
        value: &'a str,
        namespace: Option<&'a str>,
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
    Listener(RefCell<Option<ListenerCb<'a>>>),
    Any(BumpBox<'a, dyn AnyValue>),
    None,
}

type ListenerCb<'a> = BumpBox<'a, dyn FnMut(Event<dyn Any>) + 'a>;

impl<'a> AttributeValue<'a> {
    pub fn new_listener<T: 'static>(
        cx: &'a ScopeState,
        mut callback: impl FnMut(Event<T>) + 'a,
    ) -> AttributeValue<'a> {
        let boxed: BumpBox<'a, dyn FnMut(_) + 'a> = unsafe {
            BumpBox::from_raw(cx.bump().alloc(move |event: Event<dyn Any>| {
                if let Ok(data) = event.data.downcast::<T>() {
                    callback(Event {
                        propogates: event.propogates,
                        data,
                    })
                }
            }))
        };

        AttributeValue::Listener(RefCell::new(Some(boxed)))
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
            (Self::Any(l0), Self::Any(r0)) => l0.any_cmp(r0.as_ref()),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl<'a> AttributeValue<'a> {
    pub fn matches_type(&self, other: &'a AttributeValue<'a>) -> bool {
        matches!(
            (self, other),
            (Self::Text(_), Self::Text(_))
                | (Self::Float(_), Self::Float(_))
                | (Self::Int(_), Self::Int(_))
                | (Self::Bool(_), Self::Bool(_))
                | (Self::Listener(_), Self::Listener(_))
                | (Self::Any(_), Self::Any(_))
        )
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
