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

/// A table of static string literals shared by one `rsx!` body.
///
/// `rsx!` emits a single table type per body and refers to entries through [`Str`], so a body
/// with many literals defines one type and one impl instead of one pair per literal.
pub trait StaticStrings {
    /// The literals, indexed by [`Str`]'s const parameter.
    const STRINGS: &'static [&'static str];
}

/// The `I`th entry of the [`StaticStrings`] table `S`, usable as both a [`StaticText`] node and a
/// [`StaticAttributeValue`](super::StaticAttributeValue).
pub struct Str<S, const I: usize>(PhantomData<S>);

impl<S: StaticStrings, const I: usize> StaticText for Str<S, I> {
    const TEXT: &'static str = S::STRINGS[I];
}

impl<S: StaticStrings, const I: usize> super::StaticAttributeValue for Str<S, I> {
    const VALUE: &'static str = S::STRINGS[I];
}

/// A static text view.
pub struct StaticTextBuilder<T>(#[doc(hidden)] pub PhantomData<T>);

impl<T: StaticText> ViewTemplate for StaticTextBuilder<T> {
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::StaticText(T::TEXT);
    const HAS_DYNAMIC: bool = false;
}

impl<T: StaticText> View for StaticTextBuilder<T> {}
