/// The primary entrypoint for our build + optimize + bundle engine
///
/// Handles multiple ongoing tasks and allows you to queue up builds from interactive and non-interactive contexts
///
/// Uses a request -> response architecture that allows you to monitor the progress with an optional message
/// receiver.
mod builder;
mod request;
mod result;

mod assets;
mod bundle;
mod cargo;
mod handle;
mod platform;
mod prepare_html;
mod profiles;
mod progress;
mod web;

use crate::Result;
use crate::{assets::AssetManifest, dioxus_crate::DioxusCrate};

use crate::build::BuildArgs;
pub use builder::*;
pub use platform::*;
pub use request::*;
pub use result::*;
