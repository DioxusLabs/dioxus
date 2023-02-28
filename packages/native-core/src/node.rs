use rustc_hash::{FxHashMap, FxHashSet};
use shipyard::Component;
use std::{any::Any, fmt::Debug};

#[derive(Debug, Clone, Default)]
pub struct ElementNode<V: FromAnyValue = ()> {
    pub tag: String,
    pub namespace: Option<String>,
    pub attributes: FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue<V>>,
    pub listeners: FxHashSet<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TextNode {
    pub text: String,
    pub listeners: FxHashSet<String>,
}

impl TextNode {
    pub fn new(text: String) -> Self {
        Self {
            text,
            listeners: Default::default(),
        }
    }
}

/// A type of node with data specific to the node type. The types are a subset of the [VNode] types.
#[derive(Debug, Clone, Component)]
pub enum NodeType<V: FromAnyValue = ()> {
    Text(TextNode),
    Element(ElementNode<V>),
    Placeholder,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct OwnedAttributeDiscription {
    pub name: String,
    pub namespace: Option<String>,
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

pub trait FromAnyValue: Clone + 'static {
    fn from_any_value(value: &dyn Any) -> Self;
}

impl FromAnyValue for () {
    fn from_any_value(_: &dyn Any) -> Self {}
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

#[cfg(feature = "dioxus")]
impl<V: FromAnyValue> From<dioxus_core::BorrowedAttributeValue<'_>> for OwnedAttributeValue<V> {
    fn from(value: dioxus_core::BorrowedAttributeValue<'_>) -> Self {
        match value {
            dioxus_core::BorrowedAttributeValue::Text(text) => Self::Text(text.to_string()),
            dioxus_core::BorrowedAttributeValue::Float(float) => Self::Float(float),
            dioxus_core::BorrowedAttributeValue::Int(int) => Self::Int(int),
            dioxus_core::BorrowedAttributeValue::Bool(bool) => Self::Bool(bool),
            dioxus_core::BorrowedAttributeValue::Any(any) => Self::Custom(V::from_any_value(any.as_any())),
            dioxus_core::BorrowedAttributeValue::None => panic!("None attribute values result in removing the attribute, not converting it to a None value.")
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
