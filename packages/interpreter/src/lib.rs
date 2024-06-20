#![allow(clippy::empty_docs)]
#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

/// The base class that the JS channel will extend
pub static INTERPRETER_JS: &str = include_str!("./js/core.js");

/// The code explicitly for desktop/liveview that bridges the eval gap between the two
pub static NATIVE_JS: &str = include_str!("./js/native.js");

/// The code explicitly for desktop/liveview that bridges the eval gap between the two
pub static HYDRATE_JS: &str = include_str!("./js/hydrate.js");

#[cfg(all(feature = "binary-protocol", feature = "sledgehammer"))]
mod write_native_mutations;

#[cfg(all(feature = "binary-protocol", feature = "sledgehammer"))]
pub use write_native_mutations::*;

#[cfg(feature = "sledgehammer")]
pub mod unified_bindings;

#[cfg(feature = "sledgehammer")]
pub use unified_bindings::*;

#[cfg(feature = "eval")]
pub mod eval;

// Common bindings for minimal usage.
#[cfg(all(feature = "minimal_bindings", feature = "webonly"))]
pub mod minimal_bindings {
    use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

    /// Some useful snippets that we use to share common functionality between the different platforms we support.
    ///
    /// This maintains some sort of consistency between web, desktop, and liveview
    #[wasm_bindgen(module = "/src/js/common.js")]
    extern "C" {
        /// Set the attribute of the node
        pub fn setAttributeInner(node: JsValue, name: &str, value: JsValue, ns: Option<&str>);

        /// Roll up all the values from the node into a JS object that we can deserialize
        pub fn collectFormValues(node: JsValue) -> JsValue;
    }
}
