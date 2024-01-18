//! This module contains the `launch` function, which is the main entry point for dioxus fullstack

use std::any::Any;

use dioxus_lib::prelude::{Element, VirtualDom};

pub use crate::Config;

/// Launch a fullstack app with the given root component, contexts, and config.
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Config,
) {
    let virtual_dom_factory = move || {
        let mut vdom = VirtualDom::new(root);
        for context in &contexts {
            vdom.insert_any_root_context(context());
        }
        vdom
    };
    #[cfg(feature = "ssr")]
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            platform_config.launch_server(virtual_dom_factory).await;
        });
    #[cfg(not(feature = "ssr"))]
    {
        #[cfg(feature = "web")]
        platform_config.launch_web(virtual_dom_factory);
        #[cfg(feature = "desktop")]
        platform_config.launch_desktop(virtual_dom_factory);
    }
}
