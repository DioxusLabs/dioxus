#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![warn(missing_docs)]

mod any_props;
mod arena;
mod diff;
mod effect;
mod error_boundary;
mod events;
mod fragment;
mod generational_box;
mod global_context;
mod mutations;
mod nodes;
mod properties;
mod render_signal;
mod runtime;
mod scheduler;
mod scope_arena;
mod scope_context;
mod scopes;
mod tasks;
mod virtual_dom;

pub(crate) mod innerlude {
    pub(crate) use crate::any_props::*;
    pub use crate::arena::*;
    pub(crate) use crate::effect::*;
    pub use crate::error_boundary::*;
    pub use crate::events::*;
    pub use crate::fragment::*;
    pub use crate::generational_box::*;
    pub use crate::global_context::*;
    pub use crate::mutations::*;
    pub use crate::nodes::*;
    pub use crate::properties::*;
    pub use crate::runtime::{Runtime, RuntimeGuard};
    pub use crate::scheduler::*;
    pub use crate::scopes::*;
    pub use crate::tasks::*;
    pub use crate::virtual_dom::*;

    /// An [`Element`] is a possibly-none [`VNode`] created by calling `render` on [`Scope`] or [`ScopeState`].
    ///
    /// An Errored [`Element`] will propagate the error to the nearest error boundary.
    pub type Element = Option<VNode>;

    /// A [`Component`] is a function that takes [`Properties`] and returns an [`Element`].
    pub type Component<P = ()> = fn(P) -> Element;
}

pub use crate::innerlude::{
    fc_to_builder, generation, schedule_update, schedule_update_any, use_hook, vdom_is_rendering,
    AnyValue, Attribute, AttributeValue, CapturedError, Component, ComponentFunction, DynamicNode,
    Element, ElementId, Event, Fragment, HasAttributes, IntoDynNode, Mutation, Mutations,
    NoOpMutations, Properties, RenderReturn, Runtime, ScopeId, ScopeState, Task, Template,
    TemplateAttribute, TemplateNode, VComponent, VNode, VNodeInner, VPlaceholder, VText,
    VirtualDom, WriteMutations,
};

/// The purpose of this module is to alleviate imports of many common types
///
/// This includes types like [`Element`], and [`Component`].
pub mod prelude {
    pub use crate::innerlude::{
        consume_context, consume_context_from_scope, current_owner, current_scope_id,
        fc_to_builder, generation, has_context, needs_update, needs_update_any, parent_scope,
        provide_context, provide_root_context, queue_effect, remove_future, schedule_update,
        schedule_update_any, spawn, spawn_forever, spawn_isomorphic, suspend, try_consume_context,
        use_after_render, use_before_render, use_drop, use_error_boundary, use_hook,
        use_hook_with_cleanup, wait_for_next_render, with_owner, AnyValue, Attribute, Component,
        ComponentFunction, Element, ErrorBoundary, Event, EventHandler, Fragment, HasAttributes,
        IntoAttributeValue, IntoDynNode, OptionStringFromMarker, Properties, Runtime, RuntimeGuard,
        ScopeId, ScopeState, SuperFrom, SuperInto, Task, Template, TemplateAttribute, TemplateNode,
        Throw, VNode, VNodeInner, VirtualDom,
    };
}
