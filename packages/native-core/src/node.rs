use crate::{state::State, tree::NodeId};
use dioxus_core::{AnyValueContainer, AttributeValue, ElementId};
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::Debug;

/// The node is stored client side and stores only basic data about the node.
#[derive(Debug, Clone)]
pub struct Node<S: State> {
    /// The transformed state of the node.
    pub state: S,
    /// The raw data for the node
    pub node_data: NodeData,
}

#[derive(Debug, Clone)]
pub struct NodeData {
    /// The id of the node
    pub node_id: NodeId,
    /// The id of the node in the vdom.
    pub element_id: Option<ElementId>,
    /// Additional inforation specific to the node type
    pub node_type: NodeType,
}

/// A type of node with data specific to the node type. The types are a subset of the [VNode] types.
#[derive(Debug, Clone)]
pub enum NodeType {
    Text {
        text: String,
    },
    Element {
        tag: String,
        namespace: Option<String>,
        attributes: FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue>,
        listeners: FxHashSet<String>,
    },
    Placeholder,
}

impl<S: State> Node<S> {
    pub(crate) fn new(node_type: NodeType) -> Self {
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
pub struct OwnedAttributeView<'a> {
    /// The discription of the attribute.
    pub attribute: &'a OwnedAttributeDiscription,

    /// The value of the attribute.
    pub value: &'a OwnedAttributeValue,
}

#[derive(Clone)]
pub enum OwnedAttributeValue {
    Text(String),
    Float(f64),
    Int(i64),
    Bool(bool),
    Any(AnyValueContainer),
    None,
}

impl Debug for OwnedAttributeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(arg0) => f.debug_tuple("Text").field(arg0).finish(),
            Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
            Self::Int(arg0) => f.debug_tuple("Int").field(arg0).finish(),
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
            Self::Any(_) => f.debug_tuple("Any").finish(),
            Self::None => write!(f, "None"),
        }
    }
}

impl From<AttributeValue<'_>> for OwnedAttributeValue {
    fn from(value: AttributeValue<'_>) -> Self {
        match value {
            AttributeValue::Text(text) => Self::Text(text.to_string()),
            AttributeValue::Float(float) => Self::Float(float),
            AttributeValue::Int(int) => Self::Int(int),
            AttributeValue::Bool(bool) => Self::Bool(bool),
            AttributeValue::Any(any) => Self::Any(any.borrow().as_ref().unwrap().clone()),
            AttributeValue::None => Self::None,
            _ => Self::None,
        }
    }
}

impl OwnedAttributeValue {
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

    pub fn as_none(&self) -> Option<()> {
        match self {
            OwnedAttributeValue::None => Some(()),
            _ => None,
        }
    }
}
