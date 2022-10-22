use std::{
    any::Any,
    fmt::{Arguments, Display, Formatter},
};

use bumpalo::Bump;

/// Possible values for an attribute
#[derive(Clone, Copy)]
pub enum AttributeValue<'a> {
    Text(&'a str),
    Float32(f32),
    Bool(bool),
    Any(&'a dyn AnyAttributeValue),
}

// #[cfg(feature = "serialize")]

impl<'a> PartialEq for AttributeValue<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(l0), Self::Text(r0)) => l0 == r0,
            (Self::Float32(l0), Self::Float32(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            // (Self::Any(l0), Self::Any(r0)) => l0.cmp(r0),
            _ => false,
        }
    }
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

/// A trait that allows for comparing two values of the same type through the Any trait
///
/// Defaults to false if the types are not the same
///
/// This is an implicit trait, so any value that is 'static and PartialEq can be used directly
///
/// If you want to override the default behavior, you should implement PartialEq through a wrapper type
pub trait AnyAttributeValue: Any {
    /// Perform a comparison between two values
    fn cmp_any(&self, _other: &dyn Any) -> bool {
        false
    }
}

impl<T: Any + PartialEq> AnyAttributeValue for T {
    fn cmp_any(&self, other: &dyn Any) -> bool {
        match other.downcast_ref::<T>() {
            Some(t) => self == t,
            None => false,
        }
    }
}
