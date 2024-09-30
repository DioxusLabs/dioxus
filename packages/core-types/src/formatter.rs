use std::borrow::Cow;

/// Take this type and format it into a Cow<'static, str>
///
/// This trait exists so libraries like manganis can implement this type for asssets without depending
/// on dioxus-core, which can be heavy in proc macros and build scripts.
///
/// We don't want a blanket impl for T: Display because that might conflict for the other integral data
/// types of AttributeValue
///
/// Todo: we might be able to specialize without this just with Display.
pub trait DioxusFormattable {
    fn format(&self) -> Cow<'static, str>;
}
