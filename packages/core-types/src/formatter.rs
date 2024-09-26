use std::borrow::Cow;

/// Take this type and format it into a Cow<'static, str>
///
/// This trait exists so libraries like manganis can implement this type for assets without depending
/// on dioxus-core, which can be heavy in proc macros and build scripts.
///
/// We don't want a blanket impl for T: Display because that might conflict for the other integral data
/// types of AttributeValue
pub trait DioxusFormattable {
    fn format(&self) -> Cow<'static, str>;
}

impl DioxusFormattable for &'static str {
    fn format(&self) -> Cow<'static, str> {
        self.into()
    }
}

impl DioxusFormattable for String {
    fn format(&self) -> Cow<'static, str> {
        self.into()
    }
}

impl DioxusFormattable for Arguments<'_> {
    fn format(&self) -> Cow<'static, str> {
        self.to_string().into()
    }
}

/// A marker trait that automatically implements [`DioxusFormattable`] through the display impl
pub trait DioxusFormattableThroughDisplay: Display {}

impl<T: DioxusFormattableThroughDisplay> DioxusFormattable for T {
    fn format(&self) -> Cow<'static, str> {
        self.to_string().into()
    }
}
