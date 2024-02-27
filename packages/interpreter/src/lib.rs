#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub static INTERPRETER_JS: &str = include_str!("./gen/interpreter.js");
pub static COMMON_JS: &str = include_str!("./gen/common.js");

#[cfg(all(feature = "binary-protocol", feature = "sledgehammer"))]
mod write_native_mutations;

#[cfg(all(feature = "binary-protocol", feature = "sledgehammer"))]
pub use write_native_mutations::*;

// Common bindings for minimal usage.
#[cfg(all(feature = "minimal_bindings", feature = "webonly"))]
pub mod minimal_bindings {
    use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

    /// Some useful snippets that we use to share common functionality between the different platforms we support.
    ///
    /// This maintains some sort of consistency between web, desktop, and liveview
    #[wasm_bindgen(module = "/src/common_exported.js")]
    extern "C" {
        /// Set the attribute of the node
        pub fn setAttributeInner(node: JsValue, name: &str, value: JsValue, ns: Option<&str>);

        pub fn collectFormValues(node: JsValue) -> JsValue;
    }
}

#[cfg(feature = "sledgehammer")]
pub mod unified_bindings;
