//! The primary entrypoint for our build + optimize + bundle engine
//!
//! Handles multiple ongoing tasks and allows you to queue up builds from interactive and non-interactive contexts
//!
//! Uses a request -> response architecture that allows you to monitor the progress with an optional message
//! receiver.
//!
//!
//! Targets
//! - Request
//! - State
//! - Bundle
//! - Handle

mod builder;
mod patch;
mod platform;
mod prerender;
mod progress;
mod request;
mod verify;
mod web;

pub(crate) use builder::*;
pub(crate) use patch::*;
pub(crate) use platform::*;
pub(crate) use progress::*;
pub(crate) use request::*;
