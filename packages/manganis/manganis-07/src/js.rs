use const_serialize_07::SerializeConst;

use crate::{AssetOptions, AssetOptionsBuilder, AssetVariant};

#[derive(
    Debug,
    Eq,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct JsAssetOptions {
    minify: bool,
    preload: bool,
    static_head: bool,
}

impl Default for JsAssetOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl JsAssetOptions {
    pub const fn new() -> AssetOptionsBuilder<JsAssetOptions> {
        AssetOptions::js()
    }

    pub const fn default() -> Self {
        Self {
            preload: false,
            minify: true,
            static_head: false,
        }
    }

    pub const fn preloaded(&self) -> bool {
        self.preload
    }

    pub const fn static_head(&self) -> bool {
        self.static_head
    }

    pub const fn minified(&self) -> bool {
        self.minify
    }
}

impl AssetOptions {
    pub const fn js() -> AssetOptionsBuilder<JsAssetOptions> {
        AssetOptionsBuilder::variant(JsAssetOptions::default())
    }
}

impl AssetOptionsBuilder<JsAssetOptions> {
    #[allow(unused)]
    pub const fn with_minify(mut self, minify: bool) -> Self {
        self.variant.minify = minify;
        self
    }

    #[allow(unused)]
    pub const fn with_static_head(mut self, static_head: bool) -> Self {
        self.variant.static_head = static_head;
        self
    }

    #[allow(unused)]
    pub const fn with_preload(mut self, preload: bool) -> Self {
        self.variant.preload = preload;
        self
    }

    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: self.add_hash,
            variant: AssetVariant::Js(self.variant),
        }
    }
}
