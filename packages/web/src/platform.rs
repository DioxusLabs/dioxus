pub mod launch {
    use std::any::Any;

    use dioxus_core::prelude::*;

    pub type Config = crate::Config;

    pub fn launch(
        root: fn() -> Element,
        contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send>>,
        platform_config: Config,
    ) {
        wasm_bindgen_futures::spawn_local(async move {
            let mut vdom = VirtualDom::new(root);
            for context in contexts {
                vdom.insert_any_root_context(context());
            }
            crate::run(vdom, platform_config).await;
        });
    }
}
