use crate::{state::State, tree::NodeId};
use dioxus_core::{AnyValue, BorrowedAttributeValue, ElementId};
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::Debug;

/// The node is stored client side and stores only basic data about the node.
#[derive(Debug, Clone)]
pub struct Node<S: State<V>, V: FromAnyValue + 'static = ()> {
    /// The transformed state of the node.
    pub state: S,
    /// The raw data for the node
    pub node_data: NodeData<V>,
}

#[derive(Debug, Clone)]
pub struct NodeData<V: FromAnyValue = ()> {
    /// The id of the node
    pub node_id: NodeId,
    /// The id of the node in the vdom.
    pub element_id: Option<ElementId>,
    /// Additional inforation specific to the node type
    pub node_type: NodeType<V>,
}

/// A type of node with data specific to the node type. The types are a subset of the [VNode] types.
#[derive(Debug, Clone)]
pub enum NodeType<V: FromAnyValue = ()> {
    Text {
        text: String,
    },
    Element {
        tag: String,
        namespace: Option<String>,
        attributes: FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue<V>>,
        listeners: FxHashSet<String>,
    },
    Placeholder,
}

impl<S: State<V>, V: FromAnyValue> Node<S, V> {
    pub(crate) fn new(node_type: NodeType<V>) -> Self {
        Node {
            state: S::default(),
            node_data: NodeData {
                element_id: None,
                node_type,
                node_id: NodeId(0),
            },
        }
    }

    /// get the mounted id of the node
    pub fn mounted_id(&self) -> Option<ElementId> {
        self.node_data.element_id
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct OwnedAttributeDiscription {
    pub name: String,
    pub namespace: Option<String>,
    pub volatile: bool,
}

/// An attribute on a DOM node, such as `id="my-thing"` or
/// `href="https://example.com"`.
#[derive(Clone, Copy, Debug)]
pub struct OwnedAttributeView<'a, V: FromAnyValue = ()> {
    /// The discription of the attribute.
    pub attribute: &'a OwnedAttributeDiscription,

    /// The value of the attribute.
    pub value: &'a OwnedAttributeValue<V>,
}

#[derive(Clone)]
pub enum OwnedAttributeValue<V: FromAnyValue = ()> {
    Text(String),
    Float(f64),
    Int(i64),
    Bool(bool),
    Custom(V),
}

pub trait FromAnyValue: Clone {
    fn from_any_value(value: &dyn AnyValue) -> Self;
}

impl FromAnyValue for () {
    fn from_any_value(_: &dyn AnyValue) -> Self {}
}

impl<V: FromAnyValue> Debug for OwnedAttributeValue<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(arg0) => f.debug_tuple("Text").field(arg0).finish(),
            Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
            Self::Int(arg0) => f.debug_tuple("Int").field(arg0).finish(),
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
            Self::Custom(_) => f.debug_tuple("Any").finish(),
        }
    }
}

impl<V: FromAnyValue> From<BorrowedAttributeValue<'_>> for OwnedAttributeValue<V> {
    fn from(value: BorrowedAttributeValue<'_>) -> Self {
        match value {
            BorrowedAttributeValue::Text(text) => Self::Text(text.to_string()),
            BorrowedAttributeValue::Float(float) => Self::Float(float),
            BorrowedAttributeValue::Int(int) => Self::Int(int),
            BorrowedAttributeValue::Bool(bool) => Self::Bool(bool),
            BorrowedAttributeValue::Any(any) => Self::Custom(V::from_any_value(&*any)),
            BorrowedAttributeValue::None => panic!("None attribute values result in removing the attribute, not converting it to a None value.")
        }
    }
}

impl<V: FromAnyValue> OwnedAttributeValue<V> {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            OwnedAttributeValue::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            OwnedAttributeValue::Float(float) => Some(*float),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            OwnedAttributeValue::Int(int) => Some(*int),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            OwnedAttributeValue::Bool(bool) => Some(*bool),
            _ => None,
        }
    }

    pub fn as_custom(&self) -> Option<&V> {
        match self {
            OwnedAttributeValue::Custom(custom) => Some(custom),
            _ => None,
        }
    }
}
