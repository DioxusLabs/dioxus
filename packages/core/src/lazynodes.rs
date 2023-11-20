//! Support for storing lazy-nodes on the stack
//!
//! This module provides support for a type called `LazyNodes` which is a micro-heap located on the stack to make calls
//! to `rsx!` more efficient.
//!
//! To support returning rsx! from branches in match statements, we need to use dynamic dispatch on [`ScopeState`] closures.
//!
//! This can be done either through boxing directly, or by using dynamic-sized-types and a custom allocator. In our case,
//! we build a tiny alloactor in the stack and allocate the closure into that.
//!
//! The logic for this was borrowed from <https://docs.rs/stack_dst/0.6.1/stack_dst/>. Unfortunately, this crate does not
//! support non-static closures, so we've implemented the core logic of `ValueA` in this module.

#[allow(unused_imports)]
use smallbox::{smallbox, space::S16, SmallBox};

use crate::{innerlude::VNode, ScopeState};

/// A concrete type provider for closures that build [`VNode`] structures.
///
/// This struct wraps lazy structs that build [`VNode`] trees Normally, we cannot perform a blanket implementation over
/// closures, but if we wrap the closure in a concrete type, we can use it for different branches in matching.
///
///
/// ```rust, ignore
/// LazyNodes::new(|f| {
///        static TEMPLATE: dioxus::core::Template = dioxus::core::Template {
///         name: "main.rs:5:5:20", // Source location of the template for hot reloading
///         roots: &[
///             dioxus::core::TemplateNode::Element {
///                 tag: dioxus_elements::div::TAG_NAME,
///                 namespace: dioxus_elements::div::NAME_SPACE,
///                 attrs: &[],
///                 children: &[],
///             },
///         ],
///         node_paths: &[],
///         attr_paths: &[],
///     };
///     dioxus::core::VNode {
///         parent: None,
///         key: None,
///         template: std::cell::Cell::new(TEMPLATE),
///         root_ids: dioxus::core::exports::bumpalo::collections::Vec::with_capacity_in(
///                 1usize,
///                 f.bump(),
///             )
///             .into(),
///         dynamic_nodes: f.bump().alloc([]),
///         dynamic_attrs: f.bump().alloc([]),
///     })
/// }
/// ```
///
/// Find more information about how to construct [`VNode`] at <https://dioxuslabs.com/learn/0.4/contributing/walkthrough_readme#the-rsx-macro>

pub struct LazyNodes<'a, 'b> {
    #[cfg(not(miri))]
    inner: SmallBox<dyn FnMut(&'a ScopeState) -> VNode<'a> + 'b, S16>,

    #[cfg(miri)]
    inner: Box<dyn FnMut(&'a ScopeState) -> VNode<'a> + 'b>,
}

impl<'a, 'b> LazyNodes<'a, 'b> {
    /// Create a new [`LazyNodes`] closure, optimistically placing it onto the stack.
    ///
    /// If the closure cannot fit into the stack allocation (16 bytes), then it
    /// is placed on the heap. Most closures will fit into the stack, and is
    /// the most optimal way to use the creation function.
    pub fn new(val: impl FnOnce(&'a ScopeState) -> VNode<'a> + 'b) -> Self {
        // there's no way to call FnOnce without a box, so we need to store it in a slot and use static dispatch
        let mut slot = Some(val);

        Self {
            #[cfg(not(miri))]
            inner: smallbox!(move |f| {
                let val = slot.take().expect("cannot call LazyNodes twice");
                val(f)
            }),

            #[cfg(miri)]
            inner: Box::new(move |f| {
                let val = slot.take().expect("cannot call LazyNodes twice");
                val(f)
            }),
        }
    }

    /// Call the closure with the given factory to produce real [`VNode`].
    ///
    /// ```rust, ignore
    /// let f = LazyNodes::new(/* Closure for creating VNodes */);
    ///
    /// let node = f.call(cac);
    /// ```
    #[must_use]
    pub fn call(mut self, f: &'a ScopeState) -> VNode<'a> {
        (self.inner)(f)
    }
}
