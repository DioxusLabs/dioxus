pub static INTERPRETER_JS: &str = include_str!("./interpreter.js");
pub static COMMON_JS: &str = include_str!("./common.js");

#[cfg(feature = "sledgehammer")]
mod sledgehammer_bindings;
#[cfg(feature = "sledgehammer")]
pub use sledgehammer_bindings::*;

#[cfg(feature = "web")]
mod bindings;

#[cfg(feature = "web")]
pub use bindings::Interpreter;

// Common bindings for minimal usage.
#[cfg(feature = "minimal_bindings")]
pub mod minimal_bindings {
    use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
    #[wasm_bindgen(module = "/src/common.js")]
    extern "C" {
        pub fn setAttributeInner(node: JsValue, name: &str, value: JsValue, ns: Option<&str>);
    }
}
