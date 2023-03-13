//! Items related to Nodes in the RealDom

use rustc_hash::{FxHashMap, FxHashSet};
use shipyard::Component;
use std::{
    any::Any,
    fmt::{Debug, Display},
};

/// A element node in the RealDom
#[derive(Debug, Clone, Default)]
pub struct ElementNode<V: FromAnyValue = ()> {
    /// The [tag](https://developer.mozilla.org/en-US/docs/Web/API/Element/tagName) of the element
    pub tag: String,
    /// The [namespace](https://developer.mozilla.org/en-US/docs/Web/API/Element/namespaceURI) of the element
    pub namespace: Option<String>,
    /// The attributes of the element
    pub attributes: FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue<V>>,
    /// The events the element is listening for
    pub listeners: FxHashSet<String>,
}

/// A text node in the RealDom
#[derive(Debug, Clone, Default)]
pub struct TextNode {
    /// The text of the node
    pub text: String,
    /// The events the node is listening for
    pub listeners: FxHashSet<String>,
}

impl TextNode {
    /// Create a new text node
    pub fn new(text: String) -> Self {
        Self {
            text,
            listeners: Default::default(),
        }
    }
}

/// A type of node with data specific to the node type.
#[derive(Debug, Clone, Component)]
pub enum NodeType<V: FromAnyValue = ()> {
    /// A text node
    Text(TextNode),
    /// An element node
    Element(ElementNode<V>),
    /// A placeholder node. This can be used as a cheaper placeholder for a node that will be created later
    Placeholder,
}

/// A discription of an attribute on a DOM node, such as `id` or `href`.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct OwnedAttributeDiscription {
    /// The name of the attribute.
    pub name: String,
    /// The namespace of the attribute used to identify what kind of attribute it is.
    ///
    /// For renderers that use HTML, this can be used to identify if the attribute is a style attribute.
    /// Instead of parsing the style attribute every time a style is changed, you can set an attribute with the `style` namespace.
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

/// The value of an attribute on a DOM node. This contains non-text values to allow users to skip parsing attribute values in some cases.
#[derive(Clone)]
pub enum OwnedAttributeValue<V: FromAnyValue = ()> {
    /// A string value. This is the most common type of attribute.
    Text(String),
    /// A floating point value.
    Float(f64),
    /// An integer value.
    Int(i64),
    /// A boolean value.
    Bool(bool),
    /// A custom value specific to the renderer
    Custom(V),
}

/// Something that can be converted from a borrowed [Any] value.
pub trait FromAnyValue: Clone + 'static {
    /// Convert from an [Any] value.
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

impl<V: FromAnyValue + Display> Display for OwnedAttributeValue<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(arg0) => f.write_str(arg0),
            Self::Float(arg0) => f.write_str(&arg0.to_string()),
            Self::Int(arg0) => f.write_str(&arg0.to_string()),
            Self::Bool(arg0) => f.write_str(&arg0.to_string()),
            Self::Custom(arg0) => f.write_str(&arg0.to_string()),
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
    /// Attempt to convert the attribute value to a string.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            OwnedAttributeValue::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Attempt to convert the attribute value to a float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            OwnedAttributeValue::Float(float) => Some(*float),
            OwnedAttributeValue::Int(int) => Some(*int as f64),
            _ => None,
        }
    }

    /// Attempt to convert the attribute value to an integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            OwnedAttributeValue::Float(float) => Some(*float as i64),
            OwnedAttributeValue::Int(int) => Some(*int),
            _ => None,
        }
    }

    /// Attempt to convert the attribute value to a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            OwnedAttributeValue::Bool(bool) => Some(*bool),
            _ => None,
        }
    }

    /// Attempt to convert the attribute value to a custom value.
    pub fn as_custom(&self) -> Option<&V> {
        match self {
            OwnedAttributeValue::Custom(custom) => Some(custom),
            _ => None,
        }
    }
}
