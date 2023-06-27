use std::{cell::RefCell, fmt::Arguments};

use bumpalo::boxed::Box as BumpBox;
use bumpalo::Bump;

use crate::{AnyValue, AttributeValue};

/// A value that can be converted into an attribute value
pub trait IntoAttributeValue<'a> {
    /// Convert into an attribute value
    fn into_value(self, bump: &'a Bump) -> AttributeValue<'a>;
}

impl<'a> IntoAttributeValue<'a> for AttributeValue<'a> {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        self
    }
}

impl<'a> IntoAttributeValue<'a> for &'a str {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Text(self)
    }
}

impl<'a> IntoAttributeValue<'a> for f64 {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Float(self)
    }
}

impl<'a> IntoAttributeValue<'a> for i64 {
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

impl<'a> IntoAttributeValue<'a> for BumpBox<'a, dyn AnyValue> {
    fn into_value(self, _: &'a Bump) -> AttributeValue<'a> {
        AttributeValue::Any(RefCell::new(Some(self)))
    }
}

impl<'a, T: IntoAttributeValue<'a>> IntoAttributeValue<'a> for Option<T> {
    fn into_value(self, bump: &'a Bump) -> AttributeValue<'a> {
        match self {
            Some(val) => val.into_value(bump),
            None => AttributeValue::None,
        }
    }
}
