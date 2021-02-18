//! <div align="center">
//!   <h1>🌗🚀 📦 Dioxus</h1>
//!   <p>
//!     <strong>A concurrent, functional, virtual DOM for Rust</strong>
//!   </p>
//! </div>
//! Dioxus: a concurrent, functional, reactive virtual dom for any renderer in Rust.
//!
//! This crate aims to maintain a uniform hook-based, renderer-agnostic UI framework for cross-platform development.
//!
//! ## Components
//! The base unit of Dioxus is the `component`. Components can be easily created from just a function - no traits required:
//! ```
//! use dioxus_core::prelude::*;
//!
//! #[derive(Properties)]
//! struct Props { name: String }
//!
//! fn Example(ctx: &mut Context<Props>) -> VNode {
//!     html! { <div> "Hello {ctx.props.name}!" </div> }
//! }
//! ```
//! Components need to take a "Context" parameter which is generic over some properties. This defines how the component can be used
//! and what properties can be used to specify it in the VNode output. Component state in Dioxus is managed by hooks - if you're new
//! to hooks, check out the hook guide in the official guide.
//!
//! Components can also be crafted as static closures, enabling type inference without all the type signature noise:
//! ```
//! use dioxus_core::prelude::*;
//!
//! #[derive(Properties)]
//! struct Props { name: String }
//!
//! static Example: FC<Props> = |ctx, props| {
//!     html! { <div> "Hello {props.name}!" </div> }
//! }
//! ```
//!
//! If the properties struct is too noisy for you, we also provide a macro that converts variadic functions into components automatically.
//! ```
//! use dioxus_core::prelude::*;
//!
//! #[fc]
//! static Example: FC = |ctx, name: String| {
//!     html! { <div> "Hello {name}!" </div> }
//! }
//! ```
//!
//! ## Hooks
//! Dioxus uses hooks for state management. Hooks are a form of state persisted between calls of the function component. Instead of
//! using a single struct to store data, hooks use the "use_hook" building block which allows the persistence of data between
//! function component renders.
//!
//! This allows functions to reuse stateful logic between components, simplify large complex components, and adopt more clear context
//! subscription patterns to make components easier to read.
//!
//! ## Supported Renderers
//! Instead of being tightly coupled to a platform, browser, or toolkit, Dioxus implements a VirtualDOM object which
//! can be consumed to draw the UI. The Dioxus VDOM is reactive and easily consumable by 3rd-party renderers via
//! the `Patch` object. See [Implementing a Renderer](docs/8-custom-renderer.md) and the `StringRenderer` classes for information
//! on how to implement your own custom renderer. We provide 1st-class support for these renderers:
//! - dioxus-desktop (via WebView)
//! - dioxus-web (via WebSys)
//! - dioxus-ssr (via StringRenderer)
//! - dioxus-liveview (SSR + StringRenderer)
//!

pub mod changelist; // An "edit phase" described by transitions and edit operations
pub mod component; // Logic for extending FC
pub mod context; // Logic for providing hook + context functionality to user components
pub mod debug_renderer; // Test harness for validating that lifecycles and diffs work appropriately
                        // pub mod diff;
                        // pub mod patch; // The diffing algorithm that builds the ChangeList
pub mod dodriodiff; // The diffing algorithm that builds the ChangeList
pub mod error; // Error type we expose to the renderers
pub mod events; // Manages the synthetic event API
pub mod hooks; // Built-in hooks
pub mod nodebuilder; // Logic for building VNodes with a direct syntax
pub mod nodes; // Logic for the VNodes
pub mod scope; // Logic for single components
pub mod validation; //  Logic for validating trees
pub mod virtual_dom; // Most fun logic starts here, manages the lifecycle and suspense

pub mod builder {
    pub use super::nodebuilder::*;
}

// types used internally that are important
pub(crate) mod innerlude {
    pub(crate) use crate::component::{Component, Properties};
    use crate::context::hooks::Hook;
    pub(crate) use crate::context::Context;
    pub(crate) use crate::error::{Error, Result};
    use crate::nodes;
    pub(crate) use crate::scope::Scope;
    pub(crate) use crate::virtual_dom::VirtualDom;
    pub(crate) use nodes::*;

    // pub use nodes::iterables::IterableNodes;
    /// This type alias is an internal way of abstracting over the static functions that represent components.

    pub type FC<P> = for<'a> fn(Context<'a>, &'a P) -> VNode<'a>;
    // pub type FC<P> = for<'a> fn(Context<'a, P>) -> VNode<'a>;

    // TODO @Jon, fix this
    // hack the VNode type until VirtualNode is fixed in the macro crate
    pub type VirtualNode<'a> = VNode<'a>;

    // Re-export the FC macro
    pub use crate as dioxus;
    pub use crate::nodebuilder as builder;
    pub use dioxus_core_macro::fc;
    pub use dioxus_html_2::html;
}

/// Re-export common types for ease of development use.
/// Essential when working with the html! macro
pub mod prelude {
    pub use crate::component::{Component, Properties};
    pub use crate::context::Context;
    use crate::nodes;
    pub use crate::virtual_dom::VirtualDom;
    pub use nodes::*;

    // pub use nodes::iterables::IterableNodes;
    /// This type alias is an internal way of abstracting over the static functions that represent components.
    pub use crate::innerlude::FC;

    // TODO @Jon, fix this
    // hack the VNode type until VirtualNode is fixed in the macro crate
    pub type VirtualNode<'a> = VNode<'a>;

    // expose our bumpalo type
    pub use bumpalo;

    // Re-export the FC macro
    pub use crate as dioxus;
    pub use crate::nodebuilder as builder;
    pub use dioxus_core_macro::fc;
    pub use dioxus_html_2::html;

    // pub use crate::diff::DiffMachine;
    pub use crate::dodriodiff::DiffMachine;

    pub use crate::hooks::*;
}
