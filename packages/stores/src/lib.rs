#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

use dioxus_core::use_hook;
use dioxus_signals::{CopyValue, MappedMutSignal, Storage};

mod impls;
pub use impls::*;
mod store;
mod subscriptions;
pub use dioxus_stores_macro::Store;
pub use store::Store;
mod scope;
pub use scope::SelectorScope;

/// Re-exports for the storage derive macro
#[doc(hidden)]
pub mod macro_helpers {
    pub use dioxus_core;
    pub use dioxus_signals;
}

pub fn use_maybe_sync_store<T, S: Storage<T>>(
    init: impl Fn() -> T,
) -> Store<T, MappedMutSignal<T, CopyValue<T, S>>> {
    use_hook(move || Store::new(init()))
}

pub fn use_store<T>(init: impl Fn() -> T) -> Store<T, MappedMutSignal<T, CopyValue<T>>> {
    use_hook(move || Store::new(init()))
}
