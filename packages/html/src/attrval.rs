//! This module is not included anywhere.
//!
//! It is a prototype for a system that supports non-string attribute values.

trait AsAttributeValue: Sized {
    fn into_attribute_value<'a>(self, cx: NodeFactory<'a>) -> AttributeValue<'a>;
}
enum AttributeValue<'a> {
    Int(i32),
    Float(f32),
    Str(&'a str),
    Bool(bool),
}
impl<'b> AsAttributeValue for Arguments<'b> {
    fn into_attribute_value<'a>(self, cx: NodeFactory<'a>) -> AttributeValue<'a> {
        todo!()
    }
}
impl AsAttributeValue for &'static str {
    fn into_attribute_value<'a>(self, cx: NodeFactory<'a>) -> AttributeValue<'a> {
        todo!()
    }
}
impl AsAttributeValue for f32 {
    fn into_attribute_value<'a>(self, cx: NodeFactory<'a>) -> AttributeValue<'a> {
        todo!()
    }
}
impl AsAttributeValue for i32 {
    fn into_attribute_value<'a>(self, cx: NodeFactory<'a>) -> AttributeValue<'a> {
        todo!()
    }
}
