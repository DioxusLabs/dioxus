use crate::builder::{ElementBuilder, IntoAttributeValue};

// Manual implementations because they contain "volatile" values

impl<'a> ElementBuilder<'a, crate::codegen::elements::input::Input> {
    pub fn value(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        todo!();
        self
    }
}

impl<'a> ElementBuilder<'a, crate::codegen::elements::select::Select> {
    pub fn value(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        todo!();
        self
    }
}

impl<'a> ElementBuilder<'a, crate::codegen::elements::option::Option> {
    pub fn selected(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        todo!();
        self
    }
}

impl<'a> ElementBuilder<'a, crate::codegen::elements::textarea::Textarea> {
    pub fn value(mut self, val: impl IntoAttributeValue<'a>) -> Self {
        todo!();
        self
    }
}
