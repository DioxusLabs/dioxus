//! <div align="center">
//!   <h1>🌗🚀 📦 Dioxus</h1>
//!   <p>
//!     <strong>A concurrent, functional, virtual DOM for Rust</strong>
//!   </p>
//! </div>
//! Dioxus: a concurrent, functional, reactive virtual dom for any renderer in Rust.
//!
//! This crate aims to maintain a hook-based, renderer-agnostic framework for cross-platform UI development.
//!
//! ## Components
//! The base unit of Dioxus is the `component`. Components can be easily created from just a function - no traits required:
//! ```
//! use dioxus_core::prelude::*;
//!
//! #[derive(Properties)]
//! struct Props { name: String }
//!
//! fn Example(ctx: Context, props: &Props) -> DomTree {
//!     html! { <div> "Hello {props.name}!" </div> }
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
//! Many people don't like the magic of proc macros, so this is entirely optional. Under-the-hood, we simply transplant the
//! function arguments into a struct, so there's very little actual magic happening.
//!
//! ```
//! use dioxus_core::prelude::*;
//!
//! #[derive_props]
//! static Example: FC = |ctx, name: &String| {
//!     html! { <div> "Hello {name}!" </div> }
//! }
//! ```
//!
//! ## Hooks
//! Dioxus uses hooks for state management. Hooks are a form of state persisted between calls of the function component. Instead of
//! using a single struct to store data, hooks use the "use_hook" building block which allows the persistence of data between
//! function component renders. Each hook stores some data in a "memory cell" and needs to be called in a consistent order.
//! This means hooks "anything with `use_x`" may not be called conditionally.
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

pub mod arena;
pub mod component; // Logic for extending FC

pub mod debug_renderer;
pub mod diff;
pub mod patch; // An "edit phase" described by transitions and edit operations // Test harness for validating that lifecycles and diffs work appropriately
               // the diffing algorithm that builds the ChangeList
pub mod error; // Error type we expose to the renderers
pub mod events; // Manages the synthetic event API
pub mod hooks; // Built-in hooks
pub mod nodebuilder; // Logic for building VNodes with a direct syntax
pub mod nodes; // Logic for the VNodes
pub mod virtual_dom; // Most fun logic starts here, manages the lifecycle and suspense

pub mod builder {
    pub use super::nodebuilder::*;
}

// types used internally that are important
pub(crate) mod innerlude {
    pub use crate::component::*;

    pub use crate::debug_renderer::*;
    pub use crate::diff::*;
    pub use crate::error::*;
    pub use crate::events::*;
    pub use crate::hooks::*;
    pub use crate::nodebuilder::*;
    pub use crate::nodes::*;
    pub use crate::patch::*;
    pub use crate::virtual_dom::*;

    pub type FC<P> = for<'scope> fn(Context<'scope>, &'scope P) -> DomTree;

    // Re-export the FC macro
    pub use crate as dioxus;
    pub use crate::nodebuilder as builder;
    pub use dioxus_core_macro::{html, rsx};
}

/// Re-export common types for ease of development use.
/// Essential when working with the html! macro
pub mod prelude {
    pub use crate::component::{fc_to_builder, Properties};
    use crate::nodes;
    pub use crate::virtual_dom::Context;
    pub use nodes::*;

    pub use crate::nodebuilder::LazyNodes;

    pub use crate::virtual_dom::NodeCtx;
    // pub use nodes::iterables::IterableNodes;
    /// This type alias is an internal way of abstracting over the static functions that represent components.
    pub use crate::innerlude::FC;

    // TODO @Jon, fix this
    // hack the VNode type until VirtualNode is fixed in the macro crate

    // expose our bumpalo type
    pub use bumpalo;
    pub use bumpalo::Bump;

    // Re-export the FC macro
    pub use crate as dioxus;
    pub use crate::nodebuilder as builder;
    // pub use dioxus_core_macro::fc;

    pub use dioxus_core_macro::{format_args_f, html, rsx, Props};

    pub use crate::component::ScopeIdx;
    pub use crate::diff::DiffMachine;

    pub use crate::debug_renderer::DebugRenderer;
    pub use crate::dioxus_main;
    pub use crate::hooks::*;
}

#[macro_export]
macro_rules! dioxus_main {
    ($i:ident) => {
        fn main() {
            todo!("this macro is a placeholder for launching a dioxus app on different platforms. \nYou probably don't want to use this, but it's okay for small apps.")
        }
    };
}
