use bumpalo::boxed::Box as BumpBox;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    fmt::Debug,
};

use crate::{ElementId, Event};

/// An attribute of the TemplateNode, created at compile time
#[derive(Debug, PartialEq, Hash, Eq, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type")
)]
pub enum TemplateAttribute<'a> {
    /// This attribute is entirely known at compile time, enabling
    Static {
        /// The name of this attribute.
        ///
        /// For example, the `href` attribute in `href="https://example.com"`, would have the name "href"
        name: &'a str,

        /// The value of this attribute, known at compile time
        ///
        /// Currently this only accepts &str, so values, even if they're known at compile time, are not known
        value: &'a str,

        /// The namespace of this attribute. Does not exist in the HTML spec
        namespace: Option<&'a str>,
    },

    /// The attribute in this position is actually determined dynamically at runtime
    ///
    /// This is the index into the dynamic_attributes field on the container VNode
    Dynamic {
        /// The index
        id: usize,
    },
}

/// An attribute on a DOM node, such as `id="my-thing"` or `href="https://example.com"`
#[derive(Debug)]
pub struct Attribute<'a> {
    /// The name of the attribute.
    pub name: &'a str,

    /// The value of the attribute
    pub value: AttributeValue<'a>,

    /// The namespace of the attribute.
    ///
    /// Doesn’t exist in the html spec. Used in Dioxus to denote “style” tags and other attribute groups.
    pub namespace: Option<&'static str>,

    /// The element in the DOM that this attribute belongs to
    pub mounted_element: Cell<ElementId>,

    /// An indication of we should always try and set the attribute. Used in controlled components to ensure changes are propagated
    pub volatile: bool,
}

/// Any of the built-in values that the Dioxus VirtualDom supports as dynamic attributes on elements
///
/// These are built-in to be faster during the diffing process. To use a custom value, use the [`AttributeValue::Any`]
/// variant.
pub enum AttributeValue<'a> {
    /// Text attribute
    Text(&'a str),

    /// A float
    Float(f64),

    /// Signed integer
    Int(i64),

    /// Boolean
    Bool(bool),

    /// A listener, like "onclick"
    Listener(RefCell<Option<ListenerCb<'a>>>),

    /// An arbitrary value that implements PartialEq and is static
    Any(RefCell<Option<BumpBox<'a, dyn AnyValue>>>),

    /// A "none" value, resulting in the removal of an attribute from the dom
    None,
}

pub type ListenerCb<'a> = BumpBox<'a, dyn FnMut(Event<dyn Any>) + 'a>;

/// Any of the built-in values that the Dioxus VirtualDom supports as dynamic attributes on elements that are borrowed
///
/// These varients are used to communicate what the value of an attribute is that needs to be updated
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(untagged))]
pub enum BorrowedAttributeValue<'a> {
    /// Text attribute
    Text(&'a str),

    /// A float
    Float(f64),

    /// Signed integer
    Int(i64),

    /// Boolean
    Bool(bool),

    /// An arbitrary value that implements PartialEq and is static
    #[cfg_attr(
        feature = "serialize",
        serde(
            deserialize_with = "deserialize_any_value",
            serialize_with = "serialize_any_value"
        )
    )]
    Any(std::cell::Ref<'a, dyn AnyValue>),

    /// A "none" value, resulting in the removal of an attribute from the dom
    None,
}

impl<'a> From<&'a AttributeValue<'a>> for BorrowedAttributeValue<'a> {
    fn from(value: &'a AttributeValue<'a>) -> Self {
        match value {
            AttributeValue::Text(value) => BorrowedAttributeValue::Text(value),
            AttributeValue::Float(value) => BorrowedAttributeValue::Float(*value),
            AttributeValue::Int(value) => BorrowedAttributeValue::Int(*value),
            AttributeValue::Bool(value) => BorrowedAttributeValue::Bool(*value),
            AttributeValue::Listener(_) => {
                panic!("A listener cannot be turned into a borrowed value")
            }
            AttributeValue::Any(value) => {
                let value = value.borrow();
                BorrowedAttributeValue::Any(std::cell::Ref::map(value, |value| {
                    &**value.as_ref().unwrap()
                }))
            }
            AttributeValue::None => BorrowedAttributeValue::None,
        }
    }
}

impl Debug for BorrowedAttributeValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(arg0) => f.debug_tuple("Text").field(arg0).finish(),
            Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
            Self::Int(arg0) => f.debug_tuple("Int").field(arg0).finish(),
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
            Self::Any(_) => f.debug_tuple("Any").field(&"...").finish(),
            Self::None => write!(f, "None"),
        }
    }
}

impl PartialEq for BorrowedAttributeValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(l0), Self::Text(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Any(l0), Self::Any(r0)) => l0.any_cmp(&**r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[cfg(feature = "serialize")]
fn serialize_any_value<S>(_: &std::cell::Ref<'_, dyn AnyValue>, _: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    panic!("Any cannot be serialized")
}

#[cfg(feature = "serialize")]
fn deserialize_any_value<'de, 'a, D>(_: D) -> Result<std::cell::Ref<'a, dyn AnyValue>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    panic!("Any cannot be deserialized")
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
            (Self::Any(l0), Self::Any(r0)) => {
                let l0 = l0.borrow();
                let r0 = r0.borrow();
                l0.as_ref().unwrap().any_cmp(&**r0.as_ref().unwrap())
            }
            _ => false,
        }
    }
}

#[doc(hidden)]
pub trait AnyValue: 'static {
    fn any_cmp(&self, other: &dyn AnyValue) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn type_id(&self) -> TypeId {
        self.as_any().type_id()
    }
}

impl<T: Any + PartialEq + 'static> AnyValue for T {
    fn any_cmp(&self, other: &dyn AnyValue) -> bool {
        if let Some(other) = other.as_any().downcast_ref() {
            self == other
        } else {
            false
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
