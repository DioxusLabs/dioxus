//! Manual implementations because they contain "volatile" values
//!
use crate::builder::{ElementBuilder, IntoAttributeValue};

impl<'a> ElementBuilder<'a, crate::builder::Input> {
    pub fn value(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr_volatile("value", val);
        self
    }
}

impl<'a, T> ElementBuilder<'a, T> {
    pub fn selected(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr_volatile("selected", val);
        self
    }
}
