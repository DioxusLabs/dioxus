#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![warn(missing_docs)]
// Coverage runs (`RUSTFLAGS="--cfg coverage_nightly" cargo +nightly fuzz coverage`)
// opt into nightly's `#[coverage(off)]` attribute so unreachable-by-design
// regions (typed-builder marker fn bodies that the dispatcher routes around)
// don't drag the coverage metric down. Stable builds see no effect.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// Debug assertions document internal invariants, but the standard
// `debug_assert*` macros leave unreachable `cfg!(debug_assertions)` branches in
// coverage builds. These compile away completely under `coverage_nightly` so the
// fuzzer coverage report tracks reachable runtime behavior.
macro_rules! dioxus_debug_assert {
    ($($arg:tt)*) => {{
        #[cfg(all(debug_assertions, not(coverage_nightly)))]
        {
            debug_assert!($($arg)*);
        }
    }};
}

macro_rules! dioxus_debug_assert_eq {
    ($($arg:tt)*) => {{
        #[cfg(all(debug_assertions, not(coverage_nightly)))]
        {
            debug_assert_eq!($($arg)*);
        }
    }};
}

macro_rules! dioxus_debug_assert_ne {
    ($($arg:tt)*) => {{
        #[cfg(all(debug_assertions, not(coverage_nightly)))]
        {
            debug_assert_ne!($($arg)*);
        }
    }};
}

mod arena;
mod diff;
mod effect;
mod error_boundary;
mod events;
mod fragment;
mod generational_box;
mod global_context;
mod launch;
mod mount;
mod mutations;
mod nodes;
mod portal;
mod properties;
mod reactive_context;
mod render_driver;
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
    #[doc(hidden)]
    pub use crate::nodes::sort_template_attributes;

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
    pub use crate::portal::*;
    pub use crate::properties::*;
    pub use crate::reactive_context::*;
    pub use crate::render_error::*;
    pub use crate::runtime::{Runtime, RuntimeGuard};
    pub use crate::scheduler::*;
    pub use crate::scopes::*;
    pub use crate::suspense::*;
    pub use crate::tasks::*;
    pub use crate::virtual_dom::*;

    pub use anyhow::Context as AnyhowContext;
    pub use anyhow::anyhow;
    // pub use anyhow::Error as AnyhowError;
    // pub type Error = CapturedError;

    /// A result type with a default error of [`CapturedError`].
    pub type Result<T, E = CapturedError> = std::result::Result<T, E>;

    /// An [`Element`] is a possibly-none [`VNode`] created by calling `render` on [`ScopeId`] or [`ScopeState`].
    ///
    /// An Errored [`Element`] will propagate the error to the nearest error boundary.
    pub type Element = std::result::Result<VNode, RenderError>;

    /// A [`Component`] is a function that takes [`Properties`] and returns an [`Element`].
    pub type Component<P = ()> = fn(P) -> Element;
}

pub use crate::innerlude::{
    AnyValue, AnyhowContext, Attribute, AttributeValue, Callback, CapturedError, Component,
    ComponentFunction, DynamicNode, Element, ElementId, ErrorBoundary, ErrorContext, Event,
    EventHandler, Fragment, HasAttributes, IntoAttributeValue, IntoDynNode, LaunchConfig,
    ListenerCallback, MarkerWrapper, MultiTargetWriter, MultiWriter, Mutation, Mutations,
    NoOpMutations, OptionStringFromMarker, Portal, PortalProps, Properties, ReactiveContext,
    RenderError, RenderTargetId, Result, Runtime, RuntimeGuard, ScopeId, ScopeState, SpawnIfAsync,
    SubscriberList, Subscribers, SuperFrom, SuperInto, SuspendedFuture, SuspenseBoundary,
    SuspenseBoundaryProps, SuspenseContext, Task, Template, TemplateAttribute, TemplateNode,
    VComponent, VNode, VNodeInner, VText, VirtualDom, WriteMutations, anyhow, consume_context,
    consume_context_from_scope, current_owner, current_scope_id, fc_to_builder, generation,
    has_context, needs_update, needs_update_any, parent_scope, provide_context,
    provide_create_error_boundary, provide_root_context, queue_effect, remove_future,
    schedule_update, schedule_update_any, spawn, spawn_forever, spawn_isomorphic, suspend,
    throw_error, try_consume_context, use_after_render, use_before_render, use_drop, use_hook,
    use_hook_with_cleanup, with_owner,
};

/// Equivalent to `Ok::<_, dioxus::CapturedError>(value)`.
///
/// This simplifies creation of an `dioxus::Result` in places where type
/// inference cannot deduce the `E` type of the result &mdash; without needing
/// to write`Ok::<_, dioxus::CapturedError>(value)`.
///
/// One might think that `dioxus::Result::Ok(value)` would work in such cases
/// but it does not.
///
/// ```console
/// error[E0282]: type annotations needed for `std::result::Result<i32, E>`
///   --> src/main.rs:11:13
///    |
/// 11 |     let _ = dioxus::Result::Ok(1);
///    |         -   ^^^^^^^^^^^^^^^^^^ cannot infer type for type parameter `E` declared on the enum `Result`
///    |         |
///    |         consider giving this pattern the explicit type `std::result::Result<i32, E>`, where the type parameter `E` is specified
/// ```
#[allow(non_snake_case)]
pub fn Ok<T>(value: T) -> Result<T, CapturedError> {
    Result::Ok(value)
}

pub use const_format;
