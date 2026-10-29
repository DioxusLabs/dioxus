//! Core building blocks for Dioxus user interfaces.
//!
//! By default the prelude includes macros, signals, hooks, stores, HTML, and
//! document APIs. The `ssr` feature is opt-in. Disable default features to use
//! only the virtual DOM and explicitly select the APIs needed.
//!
//! ```
//! # #[cfg(all(feature = "macro", feature = "signals", feature = "hooks", feature = "stores", feature = "html"))]
//! # fn example() {
//! use dioxus_core::prelude::*;
//!
//! #[derive(Clone, Store)]
//! struct Counter {
//!     value: i32,
//! }
//!
//! #[component]
//! fn App() -> Element {
//!     let count = use_signal(|| 0);
//!     rsx! { div { "{count}" } }
//! }
//! # }
//! ```

pub use dioxus_vdom;
pub use dioxus_vdom as core;
pub use dioxus_vdom_macro as core_macro;

#[cfg(feature = "document")]
pub use dioxus_document as document;
#[cfg(feature = "hooks")]
pub use dioxus_hooks as hooks;
#[cfg(feature = "html")]
pub use dioxus_html as html;
#[cfg(feature = "signals")]
pub use dioxus_signals as signals;
#[cfg(feature = "ssr")]
pub use dioxus_ssr as ssr;
#[cfg(feature = "stores")]
pub use dioxus_stores as stores;

pub mod prelude {
    pub use dioxus_vdom;
    pub use dioxus_vdom::{
        AnyhowContext, Attribute, Callback, Component, ComponentFunctionExt, Element,
        ErrorBoundary, ErrorContext, Event, EventHandler, Fragment, HasAttributes, IntoDynNode,
        Portal, PortalProps, RenderError, RenderTargetId, Result, ScopeId, SuspenseBoundary,
        SuspenseContext, VNode, VirtualDom, consume_context, provide_context, spawn, suspend,
        try_consume_context, use_drop, use_hook,
    };

    #[cfg(feature = "document")]
    pub use dioxus_document::{self as document, Meta, Stylesheet, Title};
    #[cfg(feature = "hooks")]
    pub use dioxus_hooks::*;
    #[cfg(feature = "html")]
    pub use dioxus_html::{self, html};
    #[cfg(feature = "html")]
    pub use dioxus_html::{Code, Key, Location, Modifiers, NamedKey};
    #[cfg(feature = "html")]
    pub use dioxus_html::{
        GlobalAttributesExtension, SvgAttributesExtension, elements::prelude::*, events::*,
        extensions::*, keyboard_types, traits::*,
    };
    #[cfg(feature = "signals")]
    pub use dioxus_signals::{self, *};
    #[cfg(feature = "stores")]
    pub use dioxus_stores::{self, GlobalStore, ReadStore, Store, WriteStore, store, use_store};
    #[cfg(feature = "macro")]
    #[allow(deprecated)]
    pub use dioxus_vdom_macro::{Props, component, rsx};
}
