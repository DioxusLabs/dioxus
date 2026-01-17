use crate::{AssetOptions, AssetOptionsBuilder, AssetVariant};
use const_serialize_07::SerializeConst;

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
pub struct CssAssetOptions {
    minify: bool,
    preload: bool,
    static_head: bool,
}

impl Default for CssAssetOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl CssAssetOptions {
    pub const fn new() -> AssetOptionsBuilder<CssAssetOptions> {
        AssetOptions::css()
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
    pub const fn css() -> AssetOptionsBuilder<CssAssetOptions> {
        AssetOptionsBuilder::variant(CssAssetOptions::default())
    }
}

impl AssetOptionsBuilder<CssAssetOptions> {
    pub const fn with_minify(mut self, minify: bool) -> Self {
        self.variant.minify = minify;
        self
    }

    #[allow(unused)]
    pub const fn with_static_head(mut self, static_head: bool) -> Self {
        self.variant.static_head = static_head;
        self
    }

    pub const fn with_preload(mut self, preload: bool) -> Self {
        self.variant.preload = preload;
        self
    }

    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: true,
            variant: AssetVariant::Css(self.variant),
        }
    }
}
