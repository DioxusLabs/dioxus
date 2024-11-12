use dioxus_core::*;
use std::any::Any;

pub type Config = crate::Config<axum::Router>;

/// Launches the WebView and runs the event loop, with configuration and root props.
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_configs: Vec<Box<dyn Any>>,
) -> ! {
    #[cfg(feature = "multi-thread")]
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    #[cfg(not(feature = "multi-thread"))]
    let mut builder = tokio::runtime::Builder::new_current_thread();

    let config = platform_configs
        .into_iter()
        .find_map(|cfg| cfg.downcast::<Config>().ok().map(|cfg| *cfg))
        .unwrap_or_default();

    builder.enable_all().build().unwrap().block_on(async move {
        config
            .with_virtual_dom(move || {
                let mut virtual_dom = VirtualDom::new(root);

                for context in &contexts {
                    virtual_dom.insert_any_root_context(context());
                }

                virtual_dom
            })
            .launch()
            .await;
    });

    panic!("Launching a liveview app should never return")
}
