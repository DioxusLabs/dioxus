use std::fmt::Arguments;

use bumpalo::collections::Vec as BumpVec;
use dioxus_core::{self, exports::bumpalo, Attribute, Listener, NodeFactory, ScopeState};

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

impl<'a> IntoAttributeValue<'a> for String {
    fn into_str(self, fac: NodeFactory<'a>) -> (&'a str, bool) {
        fac.raw_text(format_args!("{}", self))
    }
}

impl<'a> IntoAttributeValue<'a> for bool {
    fn into_str(self, _fac: NodeFactory<'a>) -> (&'a str, bool) {
        match self {
            true => ("true", false),
            false => ("false", false),
        }
    }
}
