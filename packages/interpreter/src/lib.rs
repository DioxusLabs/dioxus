pub static INTERPRTER_JS: &str = include_str!("./interpreter.js");

#[cfg(feature = "web")]
mod bindings;

#[cfg(feature = "web")]
pub use bindings::Interpreter;
