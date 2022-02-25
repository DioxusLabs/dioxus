//! Declarations for the `template` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Template;

/// Build a
/// [`<template>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/template)
/// element.
pub fn template(cx: &ScopeState) -> ElementBuilder<Template> {
    ElementBuilder::new(cx, Template, "template")
}

