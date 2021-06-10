//! This file handles the supporting infrastructure for the `Component` trait and `Properties` which makes it possible
//! for components to be used within Nodes.
//!
//! Note - using the builder pattern does not required the Properties trait to be implemented - the only thing that matters is
//! if the type suppports PartialEq. The Properties trait is used by the rsx! and html! macros to generate the type-safe builder
//! that ensures compile-time required and optional fields on ctx.

use crate::innerlude::FC;

pub unsafe trait Properties: PartialEq + Sized {
    type Builder;
    const CAN_BE_MEMOIZED: bool;
    fn builder() -> Self::Builder;
}

unsafe impl Properties for () {
    const CAN_BE_MEMOIZED: bool = true;
    type Builder = EmptyBuilder;

    fn builder() -> Self::Builder {
        EmptyBuilder {}
    }
}

pub struct EmptyBuilder;
impl EmptyBuilder {
    #[inline]
    pub fn build(self) -> () {
        ()
    }
}

pub fn fc_to_builder<T: Properties>(_: FC<T>) -> T::Builder {
    T::builder()
}
