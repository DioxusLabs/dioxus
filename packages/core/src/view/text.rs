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
pub struct StaticTextBuilder<T>(PhantomData<T>);

/// Create a static text view for a text marker.
#[inline]
pub const fn static_text<T: StaticText>() -> StaticTextBuilder<T> {
    StaticTextBuilder(PhantomData)
}

impl<T: StaticText> ViewTemplate for StaticTextBuilder<T> {
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::StaticText(T::TEXT);
}

impl<T: StaticText> View for StaticTextBuilder<T> {}

/// Declare a static text marker type.
#[macro_export]
macro_rules! static_text {
    ($value:literal) => {{
        struct StaticTextMarker;
        impl $crate::view::StaticText for StaticTextMarker {
            const TEXT: &'static str = $value;
        }

        $crate::view::static_text::<StaticTextMarker>()
    }};
}
