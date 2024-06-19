//! This module contains the `launch` function, which is the main entry point for dioxus fullstack

use std::{any::Any, sync::Arc};

use dioxus_lib::prelude::{Element, VirtualDom};

pub use crate::Config;

fn virtual_dom_factory(
    root: fn() -> Element,
    contexts: Arc<Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>>,
) -> impl Fn() -> VirtualDom + 'static {
    move || {
        let mut vdom = VirtualDom::new(root);
        for context in &contexts {
            vdom.insert_any_root_context(context());
        }
        vdom
    }
}

#[cfg(feature = "server")]
/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Config,
) -> ! {
    let contexts = Arc::new(contexts);
    let factory = virtual_dom_factory(root, contexts.clone());
    #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            platform_config.launch_server(factory, contexts).await;
        });

    unreachable!("Launching a fullstack app should never return")
}

#[cfg(all(not(feature = "server"), feature = "web"))]
/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>>,
    platform_config: Config,
) {
    let factory = virtual_dom_factory(root, contexts);
    let cfg = platform_config.web_cfg.hydrate(true);
    dioxus_web::launch::launch_virtual_dom(factory(), cfg)
}

#[cfg(all(not(any(feature = "server", feature = "web")), feature = "desktop"))]
/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Config,
) -> ! {
    let factory = virtual_dom_factory(root, contexts);
    let cfg = platform_config.desktop_cfg;
    dioxus_desktop::launch::launch_virtual_dom(factory(), cfg)
}

#[cfg(all(
    not(any(feature = "server", feature = "web", feature = "desktop")),
    feature = "mobile"
))]
/// Launch a fullstack app with the given root component, contexts, and config.
#[allow(unused)]
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Config,
) {
    let factory = virtual_dom_factory(root, contexts);
    let cfg = platform_config.mobile_cfg;
    dioxus_mobile::launch::launch_virtual_dom(factory(), cfg)
}
