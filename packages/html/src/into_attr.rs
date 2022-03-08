use dioxus_core::NodeFactory;
use std::fmt::Arguments;

pub trait IntoAttributeValue<'a> {
    fn into_str(self, fac: NodeFactory<'a>) -> (&'a str, bool);
}

impl<'a, 'b> IntoAttributeValue<'a> for Arguments<'b> {
    fn into_str(self, fac: NodeFactory<'a>) -> (&'a str, bool) {
        fac.raw_text(self)
    }
}

impl<'a> IntoAttributeValue<'a> for &'a str {
    fn into_str(self, _: NodeFactory<'a>) -> (&'a str, bool) {
        (self, false)
    }
}

impl<'a> IntoAttributeValue<'a> for Option<&'a str> {
    fn into_str(self, _: NodeFactory<'a>) -> (&'a str, bool) {
        todo!()
    }
}

impl<'a> IntoAttributeValue<'a> for String {
    fn into_str(self, fac: NodeFactory<'a>) -> (&'a str, bool) {
        fac.raw_text(format_args!("{}", self))
    }
}
