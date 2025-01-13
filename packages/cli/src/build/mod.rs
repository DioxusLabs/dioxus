//! The primary entrypoint for our build + optimize + bundle engine
//!
//! Handles multiple ongoing tasks and allows you to queue up builds from interactive and non-interactive contexts
//!
//! Uses a request -> response architecture that allows you to monitor the progress with an optional message
//! receiver.

mod builder;
mod bundle;
mod prerender;
mod progress;
mod request;
mod templates;
mod verify;
mod web;

pub(crate) use builder::*;
pub(crate) use bundle::*;
pub(crate) use progress::*;
pub(crate) use request::*;
