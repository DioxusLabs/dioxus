//! <div align="center">
//!   <h1>🌗🚀 📦 Dioxus</h1>
//!   <p>
//!     <strong>A concurrent, functional, virtual DOM for Rust</strong>
//!   </p>
//! </div>
//!
//! # Resources
//!
//! This overview is provides a brief introduction to Dioxus. For a more in-depth guide, make sure to check out:
//! - [Getting Started](https://dioxuslabs.com/getting-started)
//! - [Book](https://dioxuslabs.com/book)
//! - [Reference](https://dioxuslabs.com/refernce-guide)

//!
//! # Overview and Goals
//!
//! Dioxus makes it easy to quickly build complex user interfaces with Rust. Any Dioxus app can run in the web browser,
//! as a desktop app, as a mobile app, or anywhere else provided you build the right renderer.
//!
//! Dioxus is heavily inspired by React, supporting many of the same concepts:
//!
//! - Hooks for state
//! - VirtualDom & diffing
//! - Concurrency & asynchronous rendering
//! - JSX-like templating syntax
//!
//! If you know React, then you know Dioxus.
//!
//! Dioxus is *substantially* faster than many of the other Rust UI libraries (Yew/Percy) and is *significantly* faster
//! than React, competitve with InfernoJS and frameworks like Svelte/SolidJS.
//!
//! ## Brief Overview
//!
//! All Dioxus apps are built by composing functions that take in a `Scope` and `Properties` and return an `Element`. A `Scope` holds
//! relevant state data for the the currently-rendered component.
//!
//! ```rust
//! use dioxus::prelude::*;
//!
//! fn main() {
//!     dioxus::desktop::launch(App);
//! }
//!
//! fn App(cx: Scope, props: &()) -> Element {
//!     let mut count = use_state(cx, || 0);
//!
//!     cx.render(rsx!(
//!         div { "Count: {count}" }
//!         button { onclick: move |_| count += 1, "Increment" }
//!         button { onclick: move |_| count -= 1, "Decrement" }
//!     ))
//! }
//! ```
//!
//! ## Components
//!
//! We can compose these function components to build a complex app. Each new component we design must take some Properties.
//! For components with no explicit properties, we can use the `()` type. In Dioxus, all properties are memoized by default!
//!
//! ```rust
//! fn App(cx: Scope, props &()) -> Element {
//!     cx.render(rsx!(
//!         Header {
//!             title: "My App",
//!             color: "red",
//!         }
//!     ))
//! }
//! ```
//!
//! Our `Header` component takes in a `title` and a `color` property, which we delcare on an explicit `HeaderProps` struct.
//! ```
//! // The `Props` derive macro lets us add additional functionality to how props are interpreted.
//! #[derive(Props, PartialEq)]
//! struct HeaderProps {
//!     title: String,
//!     color: String,
//! }
//!
//! fn Header(cx: Scope, props: &HeaderProps) -> Element {
//!     cx.render(rsx!(
//!         div {
//!             background_color: "{props.color}"
//!             h1 { "{props.title}" }
//!         }
//!     ))
//! }
//! ```
//!
//! ## Hooks
//!
//! While components are reusable forms of UI elements, hooks are reusable forms of logic. The details of hooks are
//! somewhat complicated. In essence, hooks let us save state between renders of our components and reuse the accompanying
//! logic across different apps.
//!
//! Hooks are simply composition of other hooks. To create our first hook we can create a simple function that takes in
//! an Scope. We can then call `use_hook` on the `Scope` to get a mutable reference to the stored value.
//!
//! ```rust
//! fn use_say_hello(cx: Scope) -> &mut String {
//!     cx.use_hook(|_| "Hello".to_string(), |hook| hook)
//! }
//! ```
//!
//! If you want to extend Dioxus with some new functionality, you'll probably want to implement a new hook.
//!
//!
//! ## Features
//!
//! This overview doesn't cover everything. Make sure to check out the tutorial and reference guide on the official
//! website for more details.
//!
//! Beyond this overview, Dioxus supports:
//! - Server-side rendering
//! - Concurrent rendering (with async support)
//! - Web/Desktop/Mobile support
//! - Pre-rendering and rehydration
//! - Fragments, Portals, and Suspense
//! - Inline-styles
//! - Custom event handlers
//! - Custom elements
//! - Basic fine-grained reactivity (IE SolidJS/Svelte)
//! - and more!

// Just a heads-up, the core functionality of dioxus rests in Dioxus-Core. This crate just wraps a bunch of utilities
// together and exports their namespaces to something predicatble.
#[cfg(feature = "core")]
pub use dioxus_core as core;

#[cfg(feature = "hooks")]
pub use dioxus_hooks as hooks;

#[cfg(feature = "ssr")]
pub use dioxus_ssr as ssr;

#[cfg(feature = "web")]
pub use dioxus_web as web;

#[cfg(feature = "mobile")]
pub use dioxus_mobile as mobile;

#[cfg(feature = "desktop")]
pub use dioxus_desktop as desktop;

#[cfg(feature = "router")]
pub use dioxus_router as router;

pub mod events {
    #[cfg(feature = "html")]
    pub use dioxus_html::{on::*, KeyCode};
}

pub mod prelude {
    pub use dioxus_core::prelude::*;
    pub use dioxus_core_macro::{format_args_f, rsx, Props, Routable};
    pub use dioxus_elements::{GlobalAttributes, SvgAttributes};
    pub use dioxus_hooks::*;
    pub use dioxus_html as dioxus_elements;
}
