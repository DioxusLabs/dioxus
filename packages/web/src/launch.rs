//! This module contains the `launch` function, which is the main entry point for dioxus web

pub use crate::Config;
use dioxus_core::prelude::*;
use std::any::Any;

/// Launch the web application with the given root component, context and config
///
/// For a builder API, see `LaunchBuilder` defined in the `dioxus` crate.
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any>>>,
    platform_config: Config,
) {
    let mut vdom = VirtualDom::new(root);
    for context in contexts {
        vdom.insert_any_root_context(context());
    }
    launch_virtual_dom(vdom, platform_config)
}

/// Launch the web application with a prebuild virtual dom
///
/// For a builder API, see `LaunchBuilder` defined in the `dioxus` crate.
pub fn launch_virtual_dom(vdom: VirtualDom, platform_config: Config) {
    wasm_bindgen_futures::spawn_local(async move {
        crate::run(vdom, platform_config).await;
    });
}

/// Launch the web application with the given root component and config
pub fn launch_cfg(root: fn() -> Element, platform_config: Config) {
    launch(root, Vec::new(), platform_config);
}
