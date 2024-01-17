use dioxus_core::*;

use crate::Config;

/// The web renderer platform
pub struct WebPlatform;

impl<Props: Clone + 'static> PlatformBuilder<Props> for WebPlatform {
    type Config = Config;

    fn launch(config: CrossPlatformConfig<Props>, platform_config: Self::Config) {
        wasm_bindgen_futures::spawn_local(async move {
            crate::run_with_props(config, platform_config).await;
        });
    }
}
