//! This file handles the supporting infrastructure for the `Component` trait and `Properties` which makes it possible
//! for components to be used within Nodes.
//!
//! Note - using the builder pattern does not required the Properties trait to be implemented - the only thing that matters is
//! if the type suppports PartialEq. The Properties trait is used by the rsx! and html! macros to generate the type-safe builder
//! that ensures compile-time required and optional fields on ctx.

use crate::innerlude::FC;

pub trait Properties: Sized {
    type Builder;
    fn builder() -> Self::Builder;

    /// Memoization can only happen if the props are 'static
    /// The user must know if their props are static, but if they make a mistake, UB happens
    /// Therefore it's unsafe to memeoize.
    unsafe fn memoize(&self, other: &Self) -> bool;
}

impl Properties for () {
    type Builder = EmptyBuilder;
    fn builder() -> Self::Builder {
        EmptyBuilder {}
    }
    unsafe fn memoize(&self, _other: &Self) -> bool {
        true
    }
}
// We allow components to use the () generic parameter if they have no props. This impl enables the "build" method
// that the macros use to anonymously complete prop construction.
pub struct EmptyBuilder;
impl EmptyBuilder {
    #[inline]
    pub fn build(self) -> () {
        ()
    }
}

/// This utility function launches the builder method so rsx! and html! macros can use the typed-builder pattern
/// to initialize a component's props.
pub fn fc_to_builder<T: Properties>(_: FC<T>) -> T::Builder {
    T::builder()
}

/// Create inline fragments
/// --
///
/// Fragments capture a series of children without rendering extra nodes.
///
/// Fragments are incredibly useful when necessary, but *do* add cost in the diffing phase.
/// Try to avoid nesting fragments if you can. Infinitely nested Fragments *will* cause diffing to crash.
#[allow(non_upper_case_globals)]
pub const Fragment: FC<()> = |ctx| {
    use crate::prelude::*;
    ctx.render(LazyNodes::new(move |c| {
        crate::nodebuilder::vfragment(c, None, ctx.children())
    }))
};
