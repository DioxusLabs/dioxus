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

/// When a `pub` struct has private fields, the `Store` derive macro generates a
/// private extension trait for private field accessors. These should not be
/// callable from outside the defining module:
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
/// And the private trait itself cannot be imported:
///
/// ```compile_fail
/// mod inner {
///     use dioxus_stores::*;
///     #[derive(Store)]
///     pub struct Item { pub name: String, secret: u32 }
/// }
/// use inner::ItemPrivateStoreExt;
/// fn main() {}
/// ```
#[cfg(doc)]
mod private_field_compile_tests {}

/// Re-exports for the store derive macro
#[doc(hidden)]
pub mod macro_helpers {
    pub use dioxus_core;
    pub use dioxus_signals;
}
