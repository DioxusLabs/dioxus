//! Declarations for the `article` element.

use crate::builder::{ElementBuilder, IntoAttributeValue};
use dioxus_core::ScopeState;

pub struct Article;

/// Build a
/// [`<article>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/article)
/// element.
pub fn article(cx: &ScopeState) -> ElementBuilder<Article> {
    ElementBuilder::new(cx, Article, "article")
}

