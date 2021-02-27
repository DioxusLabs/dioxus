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
    // pub(crate) use crate::component::Properties;

    pub(crate) use crate::context::Context;
    pub(crate) use crate::error::{Error, Result};
    use crate::nodes;
    pub(crate) use crate::scope::Scope;
    pub(crate) use crate::virtual_dom::VirtualDom;
    pub(crate) use nodes::*;

    // pub use nodes::iterables::IterableNodes;
    /// This type alias is an internal way of abstracting over the static functions that represent components.

    pub type FC<P> = for<'scope> fn(Context<'scope>, &'scope P) -> DomTree;

    mod fc2 {}
    // pub type FC<'a, P: 'a> = for<'scope> fn(Context<'scope>, &'scope P) -> DomTree;
    // pub type FC<P> = for<'scope, 'r> fn(Context<'scope>, &'scope P) -> DomTree;
    // pub type FC<P> = for<'scope, 'r> fn(Context<'scope>, &'r P) -> VNode<'scope>;
    // pub type FC<P> = for<'scope, 'r> fn(Context<'scope>, &'r P) -> VNode<'scope>;
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
    // pub use crate::component::Properties;
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
    // pub use dioxus_core_macro::fc;
    pub use dioxus_core_macro::format_args_f;
    pub use dioxus_html_2::html;

    // pub use crate::diff::DiffMachine;
    pub use crate::dodriodiff::DiffMachine;

    pub use crate::hooks::*;
}

// #[macro_use]
// extern crate dioxus_core_macro;

// #[macro_use]
// extern crate fstrings;
// pub use dioxus_core_macro::format_args_f;
// macro_rules! mk_macros {( @with_dollar![$dol:tt]=>
//     $(
//         #[doc = $doc_string:literal]
//         $printlnf:ident
//             => $println:ident!($($stream:ident,)? ...)
//         ,
//     )*
// ) => (
//     $(
//         #[doc = $doc_string]
//         #[macro_export]
//         macro_rules! $printlnf {(
//             $($dol $stream : expr,)? $dol($dol args:tt)*
//         ) => (
//             $println!($($dol $stream,)? "{}", format_args_f!($dol($dol args)*))
//         )}
//     )*
// )}

// mk_macros! { @with_dollar![$]=>
//     #[doc = "Like [`print!`](https://doc.rust-lang.org/std/macro.print.html), but with basic f-string interpolation."]
//     print_f
//         => print!(...)
//     ,
//     #[doc = "Like [`println!`](https://doc.rust-lang.org/std/macro.println.html), but with basic f-string interpolation."]
//     println_f
//         => println!(...)
//     ,
//     #[doc = "Like [`eprint!`](https://doc.rust-lang.org/std/macro.eprint.html), but with basic f-string interpolation."]
//     eprint_f
//         => eprint!(...)
//     ,
//     #[doc = "Like [`eprintln!`](https://doc.rust-lang.org/std/macro.eprintln.html), but with basic f-string interpolation."]
//     eprintln_f
//         => eprintln!(...)
//     ,
//     #[doc = "Like [`format!`](https://doc.rust-lang.org/std/macro.format.html), but with basic f-string interpolation."]
//     format_f
//         => format!(...)
//     ,
//     #[doc = "Shorthand for [`format_f`]."]
//     f
//         => format!(...)
//     ,
//     #[doc = "Like [`panic!`](https://doc.rust-lang.org/std/macro.panic.html), but with basic f-string interpolation."]
//     panic_f
//         => panic!(...)
//     ,
//     #[doc = "Like [`unreachable!`](https://doc.rust-lang.org/std/macro.unreachable.html), but with basic f-string interpolation."]
//     unreachable_f
//         => unreachable!(...)
//     ,
//     #[doc = "Like [`unimplemented!`](https://doc.rust-lang.org/std/macro.unimplemented.html), but with basic f-string interpolation."]
//     unimplemented_f
//         => unimplemented!(...)
//     ,
//     #[doc = "Like [`write!`](https://doc.rust-lang.org/std/macro.write.html), but with basic f-string interpolation."]
//     write_f
//         => write!(stream, ...)
//     ,
//     #[doc = "Like [`writeln!`](https://doc.rust-lang.org/std/macro.writeln.html), but with basic f-string interpolation."]
//     writeln_f
//         => writeln!(stream, ...)
//     ,
// }
/// Like the `format!` macro for creating `std::string::String`s but for
/// `bumpalo::collections::String`.
///
/// # Examples
///
/// ```
/// use bumpalo::Bump;
///
/// let b = Bump::new();
///
/// let who = "World";
/// let s = bumpalo::format!(in &b, "Hello, {}!", who);
/// assert_eq!(s, "Hello, World!")
/// ```
#[macro_export]
macro_rules! ifmt {
    ( in $bump:expr; $fmt:literal;) => {{
        use bumpalo::core_alloc::fmt::Write;
        use $crate::prelude::bumpalo;
        let bump = $bump;
        let mut s = bumpalo::collections::String::new_in(bump);
        let args = $crate::prelude::format_args_f!($fmt);
        s.write_fmt(args);
        s
    }};
}
// ( in $bump:expr; $fmt:expr; ) => {
// $println!("{}", format_args_f!($dol($dol args)*))

// write!(&mut s, println!("{}", args));
// let _ = $crate::write_f!(&mut s, $fmt);
// s
//     use fstrings::*;
//     $crate::ifmt!(in $bump, $fmt)
// };

#[test]
fn macro_test() {
    let w = 123;
    let world = &w;
    // let g = format_args_f!("Hello {world}");

    // dbg!(g);
}
