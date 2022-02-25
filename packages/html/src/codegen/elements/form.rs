//! Declarations for the `form` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Form;

/// Build a
/// [`<form>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
/// element.
pub fn form(cx: &ScopeState) -> ElementBuilder<Form> {
    ElementBuilder::new(cx, Form, "form")
}

impl<'a> ElementBuilder<'a, Form> {
    #[inline]
    pub fn name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("name", val);
        self
    }
    #[inline]
    pub fn autocomplete(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("autocomplete", val);
        self
    }
    #[inline]
    pub fn enctype(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("enctype", val);
        self
    }
    #[inline]
    pub fn novalidate(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("novalidate", val);
        self
    }
    #[inline]
    pub fn method(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("method", val);
        self
    }
    #[inline]
    pub fn target(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("target", val);
        self
    }
    #[inline]
    pub fn action(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        self.push_attr("action", val);
        self
    }
} 
