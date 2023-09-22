#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod any_props;
mod arena;
mod bump_frame;
mod create;
mod diff;
mod dirty_scope;
mod error_boundary;
mod events;
mod fragment;
mod lazynodes;
mod mutations;
mod nodes;
mod properties;
mod runtime;
mod scheduler;
mod scope_arena;
mod scope_context;
mod scopes;
mod virtual_dom;

pub(crate) mod innerlude {
    pub use crate::arena::*;
    pub use crate::dirty_scope::*;
    pub use crate::error_boundary::*;
    pub use crate::events::*;
    pub use crate::fragment::*;
    pub use crate::lazynodes::*;
    pub use crate::mutations::*;
    pub use crate::nodes::RenderReturn;
    pub use crate::nodes::*;
    pub use crate::properties::*;
    pub use crate::runtime::{Runtime, RuntimeGuard};
    pub use crate::scheduler::*;
    pub use crate::scope_context::*;
    pub use crate::scopes::*;
    pub use crate::virtual_dom::*;

    /// An [`Element`] is a possibly-none [`VNode`] created by calling `render` on [`Scope`] or [`ScopeState`].
    ///
    /// An Errored [`Element`] will propagate the error to the nearest error boundary.
    pub type Element<'a> = Option<VNode<'a>>;

    /// A [`Component`] is a function that takes a [`Scope`] and returns an [`Element`].
    ///
    /// Components can be used in other components with two syntax options:
    /// - lowercase as a function call with named arguments (rust style)
    /// - uppercase as an element (react style)
    ///
    /// ## Rust-Style
    ///
    /// ```rust, ignore
    /// fn example(cx: Scope<Props>) -> Element {
    ///     // ...
    /// }
    ///
    /// rsx!(
    ///     example()
    /// )
    /// ```
    /// ## React-Style
    /// ```rust, ignore
    /// fn Example(cx: Scope<Props>) -> Element {
    ///     // ...
    /// }
    ///
    /// rsx!(
    ///     Example {}
    /// )
    /// ```
    pub type Component<P = ()> = fn(Scope<P>) -> Element;
}

pub use crate::innerlude::{
    fc_to_builder, vdom_is_rendering, AnyValue, Attribute, AttributeValue, BorrowedAttributeValue,
    CapturedError, Component, DynamicNode, Element, ElementId, Event, Fragment, IntoDynNode,
    LazyNodes, Mutation, Mutations, Properties, RenderReturn, Scope, ScopeId, ScopeState, Scoped,
    TaskId, Template, TemplateAttribute, TemplateNode, VComponent, VNode, VPlaceholder, VText,
    VirtualDom,
};

/// The purpose of this module is to alleviate imports of many common types
///
/// This includes types like [`Scope`], [`Element`], and [`Component`].
pub mod prelude {
    pub use crate::innerlude::{
        consume_context, consume_context_from_scope, current_scope_id, fc_to_builder, has_context,
        provide_context, provide_context_to_scope, provide_root_context, push_future,
        remove_future, schedule_update_any, spawn, spawn_forever, suspend, throw, AnyValue,
        Component, Element, Event, EventHandler, Fragment, IntoAttributeValue, LazyNodes,
        Properties, Runtime, RuntimeGuard, Scope, ScopeId, ScopeState, Scoped, TaskId, Template,
        TemplateAttribute, TemplateNode, Throw, VNode, VirtualDom,
    };
}

pub mod exports {
    //! Important dependencies that are used by the rest of the library
    //! Feel free to just add the dependencies in your own Crates.toml
    pub use bumpalo;
}
