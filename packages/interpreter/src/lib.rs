pub static INTERPRETER_JS: &str = include_str!("./interpreter.js");

#[cfg(feature = "web")]
mod sledgehammer_bindings;
#[cfg(feature = "web")]
pub use sledgehammer_bindings::*;

#[cfg(feature = "web")]
mod bindings;

#[cfg(feature = "web")]
pub use bindings::Interpreter;
