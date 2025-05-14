#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub use dioxus_desktop::*;
use dioxus_lib::prelude::*;
use std::any::Any;

/// Launch via the binding API
pub fn launch(root: fn() -> Element) {
    launch_cfg(root, vec![], vec![]);
}

pub fn launch_cfg(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
) {
    dioxus_desktop::launch::launch_cfg(root, contexts, platform_config);
}
