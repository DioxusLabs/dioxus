//! This module contains the `launch` function, which is the main entry point for dioxus fullstack

use std::any::Any;

use dioxus_lib::prelude::{Element, VirtualDom};

pub use crate::Config;

/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
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

    #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            platform_config.launch_server(virtual_dom_factory).await;
        });

    #[cfg(not(feature = "server"))]
    {
        #[cfg(feature = "web")]
        {
            // TODO: this should pull the props from the document
            let cfg = platform_config.web_cfg.hydrate(true);
            dioxus_web::launch::launch_virtual_dom(virtual_dom_factory(), cfg);
        }

        #[cfg(feature = "desktop")]
        {
            let cfg = platform_config.desktop_cfg;
            dioxus_desktop::launch::launch_virtual_dom(virtual_dom_factory(), cfg)
        }

        #[cfg(feature = "mobile")]
        {
            let cfg = platform_config.mobile_cfg;
            dioxus_mobile::launch::launch_virtual_dom(virtual_dom_factory(), cfg)
        }
    }
}
