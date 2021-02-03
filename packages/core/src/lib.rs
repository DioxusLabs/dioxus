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
//! static Example: FC<Props> = |ctx| {
//!     html! { <div> "Hello {ctx.props.name}!" </div> }
//! }
//! ```
//!
//! If the properties struct is too noisy for you, we also provide a macro that converts variadic functions into components automatically.
//! ```
//! use dioxus_core::prelude::*;
//!
//! #[functional_component]
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

pub mod nodebuilder;
pub mod nodes;
pub mod validation;
pub mod virtual_dom;

/// Re-export common types for ease of development use.
/// Essential when working with the html! macro
///
///
///
pub mod prelude {
    use crate::nodes;
    pub use crate::virtual_dom::{Context, VirtualDom};
    pub use nodes::*;
    // pub use nodes::iterables::IterableNodes;

    /// This type alias is an internal way of abstracting over the static functions that represent components.
    pub type FC<P> = for<'a> fn(&'a Context<P>) -> VNode<'a>;

    // TODO @Jon, fix this
    // hack the VNode type until VirtualNode is fixed in the macro crate
    pub type VirtualNode<'a> = VNode<'a>;

    // Re-export from the macro crate
    pub use dioxus_html_macro::html;

    // Re-export the FC macro
    pub use dioxus_core_macro::fc;
}
