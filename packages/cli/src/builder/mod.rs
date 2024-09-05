/// The primary entrypoint for our build + optimize + bundle engine
///
/// Handles multiple ongoing tasks and allows you to queue up builds from interactive and non-interactive contexts
///
/// Uses a request -> response architecture that allows you to monitor the progress with an optional message
/// receiver.
mod builder;
mod bundle;
mod cargo;
mod handle;
mod platform;
mod profiles;
mod progress;
mod request;
mod result;
mod tooling;
mod web;

use crate::build::BuildArgs;
use crate::Result;
use crate::{assets::AssetManifest, dioxus_crate::DioxusCrate};

pub use builder::*;
pub use platform::*;
pub use progress::*;
pub use request::*;
pub use result::*;
