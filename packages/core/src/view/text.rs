//! The typed static text view: [`StaticTextBuilder`] and its marker trait
//! [`StaticText`].

use std::marker::PhantomData;

use dioxus_core_template::TemplateRawTree;

use super::{View, ViewTemplate};

/// A marker for one static text node.
pub trait StaticText {
    /// Static text value.
    const TEXT: &'static str;
}

/// A static text view.
pub struct StaticTextBuilder<T>(#[doc(hidden)] pub PhantomData<T>);

impl<T: StaticText> ViewTemplate for StaticTextBuilder<T> {
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::StaticText(T::TEXT);
}

impl<T: StaticText> View for StaticTextBuilder<T> {}
