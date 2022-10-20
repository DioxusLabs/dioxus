use std::{
    any::Any,
    fmt::{Arguments, Display, Formatter},
};

use bumpalo::Bump;
use serde::{Deserialize, Serialize};

/// Possible values for an attribute
// trying to keep values at 3 bytes
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(untagged))]
#[derive(Clone, PartialEq)]
#[allow(missing_docs)]
pub enum AttributeValue<'a> {
    Text(&'a str),
    Float32(f32),
    Bool(bool),
    Any(ArbitraryAttributeValue<'a>),
}

impl<'a> Display for AttributeValue<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::fmt::Debug for AttributeValue<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
        // match self {
        //     AttributeValue::Text(s) => write!(f, "AttributeValue::Text({:?})", s),
        //     AttributeValue::Float32(f) => write!(f, "AttributeValue::Float32({:?})", f),
        //     AttributeValue::Bool(b) => write!(f, "AttributeValue::Bool({:?})", b),
        //     AttributeValue::Any(a) => write!(f, "AttributeValue::Any({:?})", a),
        // }
    }
}

/// A value that can be converted into an attribute value
pub trait IntoAttributeValue<'a> {
    /// Convert into an attribute value
    fn into_value(self, bump: &'a Bump) -> AttributeValue<'a>;
}
impl<'a> IntoAttributeValue<'a> for f32 {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Float32(self)
    }
}

impl<'a> IntoAttributeValue<'a> for bool {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Bool(self)
    }
}

impl<'a> IntoAttributeValue<'a> for &'a str {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Text(self)
    }
}

impl<'a> IntoAttributeValue<'a> for Arguments<'_> {
    fn into_value(self, bump: &'a Bump) -> AttributeValue<'a> {
        use bumpalo::core_alloc::fmt::Write;
        let mut str_buf = bumpalo::collections::String::new_in(bump);
        str_buf.write_fmt(self).unwrap();
        AttributeValue::Text(str_buf.into_bump_str())
    }
}

impl<'a, T> IntoAttributeValue<'a> for &'a T
where
    T: PartialEq,
{
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        todo!()
        // AttributeValue::Any(ArbitraryAttributeValue {
        //     value: self,
        //     cmp: |a, b| {
        //         if let Some(a) = a.as_any().downcast_ref::<T>() {
        //             if let Some(b) = b.as_any().downcast_ref::<T>() {
        //                 a == b
        //             } else {
        //                 false
        //             }
        //         } else {
        //             false
        //         }
        //     },
        // })
    }
}

// todo
#[allow(missing_docs)]
impl<'a> AttributeValue<'a> {
    pub fn is_truthy(&self) -> bool {
        match self {
            AttributeValue::Text(t) => *t == "true",
            AttributeValue::Bool(t) => *t,
            _ => false,
        }
    }

    pub fn is_falsy(&self) -> bool {
        match self {
            AttributeValue::Text(t) => *t == "false",
            AttributeValue::Bool(t) => !(*t),
            _ => false,
        }
    }
}

#[derive(Clone, Copy)]
#[allow(missing_docs)]
pub struct ArbitraryAttributeValue<'a> {
    pub value: &'a dyn Any,
    // pub value: &'a dyn AnyClone,
    // pub cmp: fn(&dyn AnyClone, &dyn AnyClone) -> bool,
}

#[cfg(feature = "serialize")]
impl<'a> Serialize for ArbitraryAttributeValue<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}
#[cfg(feature = "serialize")]
impl<'a, 'de> Deserialize<'de> for ArbitraryAttributeValue<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

impl PartialEq for ArbitraryAttributeValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        todo!()
        // (self.cmp)(self.value, other.value)
    }
}

// todo
#[allow(missing_docs)]
impl<'a> AttributeValue<'a> {
    pub fn as_text(&self) -> Option<&'a str> {
        match self {
            AttributeValue::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_float32(&self) -> Option<f32> {
        match self {
            AttributeValue::Float32(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_any(&self) -> Option<&'a ArbitraryAttributeValue> {
        match self {
            AttributeValue::Any(a) => Some(a),
            _ => None,
        }
    }
}
