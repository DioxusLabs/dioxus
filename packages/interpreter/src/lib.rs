#![allow(clippy::empty_docs)]
#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

/// The base class that the JS channel will extend
pub static INTERPRETER_JS: &str = include_str!("./js/core.js");

/// The code explicitly for desktop/liveview that bridges the eval gap between the two
pub static NATIVE_JS: &str = include_str!("./js/native.js");

/// The code that handles initializing data used for fullstack data streaming
pub static INITIALIZE_STREAMING_JS: &str = include_str!("./js/initialize_streaming.js");

#[cfg(all(feature = "binary-protocol", feature = "sledgehammer"))]
mod write_native_mutations;

#[cfg(all(feature = "binary-protocol", feature = "sledgehammer"))]
pub use write_native_mutations::*;

#[cfg(feature = "sledgehammer")]
pub mod unified_bindings;

#[cfg(feature = "sledgehammer")]
pub use unified_bindings::*;

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

    #[wasm_bindgen(module = "/src/js/hydrate.js")]
    extern "C" {
        /// Register a callback that that will be called to hydrate a node at the given id with data from the server
        pub fn register_rehydrate_chunk_for_streaming(
            closure: &wasm_bindgen::closure::Closure<dyn FnMut(Vec<u32>, js_sys::Uint8Array)>,
        );
    }

    #[wasm_bindgen(module = "/src/js/patch_console.js")]
    extern "C" {
        pub fn monkeyPatchConsole(ws: JsValue);
    }
}
