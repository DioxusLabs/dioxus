use dioxus_core::exports::bumpalo::collections::Vec as BumpVec;
use dioxus_core::{prelude::*, Attribute};

mod buildable;
mod elements;
mod global_attributes;
mod types;

pub struct NodeBuilder<'a> {
    inner: &'a ScopeState,
    children: BumpVec<'a, Element<'a>>,
    attributes: BumpVec<'a, Attribute<'a>>,
}

impl<'a> NodeBuilder<'a> {
    pub fn set_attribute_capacity(&mut self, attributes: usize) {
        self.attributes.reserve_exact(attributes);
    }

    pub fn push_attribute(&mut self, attribute: Attribute<'a>) {
        self.attributes.push(attribute);
    }

    pub fn build(self) -> Element<'a> {
        todo!()
    }
}
