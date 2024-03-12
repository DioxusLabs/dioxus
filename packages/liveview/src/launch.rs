use dioxus_core::*;
use std::any::Any;

pub type Config = crate::Config<axum::Router>;

/// Launches the WebView and runs the event loop, with configuration and root props.
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Config,
) {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            platform_config
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
}
