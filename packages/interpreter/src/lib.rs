pub static INTERPRETER_JS: &str = include_str!("./interpreter.js");

#[cfg(feature = "sledgehammer")]
mod sledgehammer_bindings;
#[cfg(feature = "sledgehammer")]
pub use sledgehammer_bindings::*;

#[cfg(feature = "web")]
mod bindings;

#[cfg(feature = "web")]
pub use bindings::Interpreter;
