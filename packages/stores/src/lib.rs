#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

mod impls;
mod store;
mod subscriptions;
pub use impls::*;
pub use store::*;
pub mod scope;

#[cfg(feature = "macro")]
pub use dioxus_stores_macro::{store, Store};

/// When a `pub` struct has fields with restricted visibility, the `Store` derive
/// macro gates their accessor methods so they can only be called from a scope
/// that matches the field's visibility. Calling a private-field accessor from
/// outside the defining module fails to compile:
///
/// ```compile_fail
/// use dioxus_stores::*;
/// mod inner {
///     use dioxus_stores::*;
///     #[derive(Store)]
///     pub struct Item { pub name: String, secret: u32 }
///     impl Item { pub fn new() -> Self { Item { name: "hi".into(), secret: 0 } } }
/// }
/// fn main() {
///     use inner::*;
///     let store = use_store(inner::Item::new);
///     let _ = store.secret();
/// }
/// ```
///
/// Likewise, private fields in the transposed struct remain private:
///
/// ```compile_fail
/// use dioxus_stores::*;
/// mod inner {
///     use dioxus_stores::*;
///     #[derive(Store)]
///     pub struct Item { pub name: String, secret: u32 }
///     impl Item { pub fn new() -> Self { Item { name: "hi".into(), secret: 0 } } }
/// }
/// fn main() {
///     use inner::*;
///     let store = use_store(inner::Item::new);
///     let _ = store.transpose().secret;
/// }
/// ```
///
/// `pub(crate)` fields remain callable anywhere inside the crate but are
/// unreachable from other crates. Arbitrary `pub(in path)` and `pub(super)`
/// visibilities are supported — the accessor is callable from exactly the
/// scope that could have named the field itself:
///
/// ```compile_fail
/// use dioxus_stores::*;
/// mod parent {
///     pub mod defining {
///         use dioxus_stores::*;
///         #[derive(Store)]
///         pub struct Item { pub(super) parent_only: u32 }
///         impl Item { pub fn new() -> Self { Self { parent_only: 0 } } }
///     }
/// }
/// fn main() {
///     use parent::defining::*;
///     let store = use_store(Item::new);
///     // parent_only is pub(super) — only callable from `parent`, not here.
///     let _ = store.parent_only();
/// }
/// ```
#[cfg(doc)]
mod private_field_compile_tests {}

/// Re-exports for the store derive macro
#[doc(hidden)]
pub mod macro_helpers {
    pub use dioxus_core;
    pub use dioxus_signals;
}
