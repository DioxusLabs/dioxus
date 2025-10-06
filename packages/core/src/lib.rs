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
mod launch;
mod mutations;
mod nodes;
mod properties;
mod reactive_context;
mod render_error;
mod root_wrapper;
mod runtime;
mod scheduler;
mod scope_arena;
mod scope_context;
mod scopes;
mod suspense;
mod tasks;
mod virtual_dom;

mod hotreload_utils;

/// Items exported from this module are used in macros and should not be used directly.
#[doc(hidden)]
pub mod internal {
    #[doc(hidden)]
    pub use crate::hotreload_utils::{
        DynamicLiteralPool, DynamicValuePool, FmtSegment, FmtedSegments, HotReloadAttributeValue,
        HotReloadDynamicAttribute, HotReloadDynamicNode, HotReloadLiteral,
        HotReloadTemplateWithLocation, HotReloadedTemplate, HotreloadedLiteral, NamedAttribute,
        TemplateGlobalKey,
    };

    #[allow(non_snake_case)]
    #[doc(hidden)]
    pub fn Err<T, E>(e: E) -> Result<T, E> {
        std::result::Result::Err(e)
    }

    pub use anyhow::__anyhow;

    #[doc(hidden)]
    pub use generational_box;
}

pub(crate) mod innerlude {
    pub(crate) use crate::any_props::*;
    pub use crate::arena::*;
    pub(crate) use crate::effect::*;
    pub use crate::error_boundary::*;
    pub use crate::events::*;
    pub use crate::fragment::*;
    pub use crate::generational_box::*;
    pub use crate::global_context::*;
    pub use crate::launch::*;
    pub use crate::mutations::*;
    pub use crate::nodes::*;
    pub use crate::properties::*;
    pub use crate::reactive_context::*;
    pub use crate::render_error::*;
    pub use crate::runtime::{Runtime, RuntimeGuard};
    pub use crate::scheduler::*;
    pub use crate::scopes::*;
    pub use crate::suspense::*;
    pub use crate::tasks::*;
    pub use crate::virtual_dom::*;

    pub use anyhow::anyhow;
    pub use anyhow::Context as AnyhowContext;
    pub use anyhow::Error;
    pub use anyhow::Result;

    /// An [`Element`] is a possibly-none [`VNode`] created by calling `render` on [`ScopeId`] or [`ScopeState`].
    ///
    /// An Errored [`Element`] will propagate the error to the nearest error boundary.
    pub type Element = std::result::Result<VNode, RenderError>;

    /// A [`Component`] is a function that takes [`Properties`] and returns an [`Element`].
    pub type Component<P = ()> = fn(P) -> Element;
}

pub use crate::innerlude::{
    anyhow, consume_context, consume_context_from_scope, current_owner, current_scope_id,
    fc_to_builder, generation, has_context, needs_update, needs_update_any, parent_scope,
    provide_context, provide_create_error_boundary, provide_root_context, queue_effect,
    remove_future, schedule_update, schedule_update_any, spawn, spawn_forever, spawn_isomorphic,
    suspend, throw_error, try_consume_context, use_after_render, use_before_render, use_drop,
    use_hook, use_hook_with_cleanup, with_owner, AnyValue, AnyhowContext, Attribute,
    AttributeValue, Callback, CapturedError, Component, ComponentFunction, DynamicNode, Element,
    ElementId, Error, ErrorBoundary, ErrorContext, Event, EventHandler, Fragment, HasAttributes,
    IntoAttributeValue, IntoDynNode, LaunchConfig, ListenerCallback, MarkerWrapper, Mutation,
    Mutations, NoOpMutations, OptionStringFromMarker, Properties, ReactiveContext, RenderError,
    Result, Runtime, RuntimeGuard, ScopeId, ScopeState, SpawnIfAsync, SubscriberList, Subscribers,
    SuperFrom, SuperInto, SuspendedFuture, SuspenseBoundary, SuspenseBoundaryProps,
    SuspenseContext, Task, Template, TemplateAttribute, TemplateNode, VComponent, VNode,
    VNodeInner, VPlaceholder, VText, VirtualDom, WriteMutations,
};

pub use anyhow::Ok;
pub use const_format;
