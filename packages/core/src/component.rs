//! This file handles the supporting infrastructure for the `Component` trait and `Properties` which makes it possible
//! for components to be used within Nodes.
//!
//! Note - using the builder pattern does not required the Properties trait to be implemented - the only thing that matters is
//! if the type suppports PartialEq. The Properties trait is used by the rsx! and html! macros to generate the type-safe builder
//! that ensures compile-time required and optional fields on props.

use crate::innerlude::FC;

pub type ScopeIdx = generational_arena::Index;

pub trait Properties: PartialEq {
    type Builder;
    type StaticOutput: Properties + 'static;
    fn builder() -> Self::Builder;
    unsafe fn into_static(self) -> Self::StaticOutput;
}

pub struct EmptyBuilder;
impl EmptyBuilder {
    pub fn build(self) -> () {
        ()
    }
}

impl Properties for () {
    type Builder = EmptyBuilder;
    type StaticOutput = ();

    fn builder() -> Self::Builder {
        EmptyBuilder {}
    }

    unsafe fn into_static(self) -> Self::StaticOutput {
        std::mem::transmute(self)
    }
}

pub fn fc_to_builder<T: Properties>(_f: FC<T>) -> T::Builder {
    T::builder()
}
