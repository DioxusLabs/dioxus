#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub static INTERPRETER_JS: &str = include_str!("./interpreter.js");
pub static COMMON_JS: &str = include_str!("./common.js");

#[cfg(feature = "sledgehammer")]
mod sledgehammer_bindings;

#[cfg(feature = "sledgehammer")]
pub use sledgehammer_bindings::*;

#[cfg(all(feature = "binary-protocol", feature = "sledgehammer"))]
mod write_native_mutations;
#[cfg(all(feature = "binary-protocol", feature = "sledgehammer"))]
pub use write_native_mutations::*;

// Common bindings for minimal usage.
#[cfg(all(feature = "minimal_bindings", feature = "webonly"))]
pub mod minimal_bindings {
    use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
    #[wasm_bindgen(module = "/src/common.js")]
    extern "C" {
        pub fn setAttributeInner(node: JsValue, name: &str, value: JsValue, ns: Option<&str>);
    }
}
