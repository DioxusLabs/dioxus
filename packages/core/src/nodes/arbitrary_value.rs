use std::{
    any::Any,
    fmt::{Arguments, Formatter},
};

use bumpalo::Bump;

/// Possible values for an attribute
#[derive(Clone, Copy)]
pub enum AttributeValue<'a> {
    /// Reference strs, most common
    Text(&'a str),
    /// Basic float values
    Float(f32),
    /// Basic Int values
    Int(i32),
    /// Basic bool values
    Bool(bool),
    /// Everything else
    Any(&'a dyn AnyAttributeValue),
}

impl<'a> PartialEq for AttributeValue<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(l0), Self::Text(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Any(l0), Self::Any(r0)) => (*l0).cmp_any(*r0),
            _ => false,
        }
    }
}

impl std::fmt::Debug for AttributeValue<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeValue::Text(s) => write!(f, "AttributeValue::Text({:?})", s),
            AttributeValue::Float(v) => write!(f, "AttributeValue::Float({:?})", v),
            AttributeValue::Int(v) => write!(f, "AttributeValue::Int({:?})", v),
            AttributeValue::Bool(b) => write!(f, "AttributeValue::Bool({:?})", b),
            AttributeValue::Any(_) => write!(f, "AttributeValue::Any()"),
        }
    }
}

/// A value that can be converted into an attribute value
pub trait IntoAttributeValue<'a> {
    /// Convert into an attribute value
    fn into_value(self, bump: &'a Bump) -> AttributeValue<'a>;
}

impl<'a> IntoAttributeValue<'a> for &'a str {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Text(self)
    }
}
impl<'a> IntoAttributeValue<'a> for f32 {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Float(self)
    }
}
impl<'a> IntoAttributeValue<'a> for i32 {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Int(self)
    }
}
impl<'a> IntoAttributeValue<'a> for bool {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Bool(self)
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
    fn cmp_any<'a>(&'a self, _other: &'a dyn AnyAttributeValue) -> bool {
        false
    }
}

impl<T: Any + PartialEq> AnyAttributeValue for T {
    fn cmp_any(&self, other: &dyn AnyAttributeValue) -> bool {
        // we can't, for whatever reason use other as &dyn Any
        let right: &dyn Any = unsafe { std::mem::transmute(other) };

        match right.downcast_ref::<T>() {
            Some(right) => self == right,
            None => false,
        }
    }
}

#[test]
fn cmp_any_works_even_though_it_transmutes() {
    // same type, same value
    let a = 2;
    let b = 2;
    assert!(a.cmp_any(&b as &dyn AnyAttributeValue));

    // same type, different value
    let a = "asds";
    let b = "asdsasd";
    assert!(!a.cmp_any(&b as &dyn AnyAttributeValue));

    // different type, different values
    let a = 123;
    let b = "asdsasd";
    assert!(!a.cmp_any(&b as &dyn AnyAttributeValue));

    // Custom structs
    #[derive(PartialEq)]
    struct CustomStruct {
        a: i32,
    }

    let a = CustomStruct { a: 1 };
    let b = CustomStruct { a: 1 };
    assert!(a.cmp_any(&b as &dyn AnyAttributeValue));

    let a = CustomStruct { a: 1 };
    let b = CustomStruct { a: 123 };
    assert!(!a.cmp_any(&b as &dyn AnyAttributeValue));

    let a = CustomStruct { a: 1 };
    let b = "asdasd";
    assert!(!a.cmp_any(&b as &dyn AnyAttributeValue));
}
