use const_serialize::SerializeConst;

use crate::GenericAssetOptions;

/// A builder for a javascript asset. This must be used in the [`mg!`] macro.#[derive(SerializeConst)]
#[derive(SerializeConst)]
pub struct JsAssetOptions {
    minify: bool,
    preload: bool,
}

impl JsAssetOptions {
    /// Create a new js asset builder
    pub const fn new() -> Self {
        Self {
            minify: true,
            preload: false,
        }
    }

    /// Sets whether the js should be minified (default: true)
    ///
    /// Minifying the js can make your site load faster by loading less data
    ///
    /// ```rust
    /// const _: &str = manganis::mg!(js("assets/script.js").minify(false));
    /// ```
    #[allow(unused)]
    pub const fn minify(self, minify: bool) -> Self {
        Self { minify, ..self }
    }

    /// Make the asset preloaded
    ///
    /// Preloading the javascript will make the javascript start to load as soon as possible. This is useful for javascript that will be used soon after the page loads or javascript that may not be used immediately, but should start loading sooner
    ///
    /// ```rust
    /// const _: manganis::ImageAsset = manganis::mg!(css("https://sindresorhus.com/github-markdown-css/github-markdown.css").preload());
    /// ```
    #[allow(unused)]
    pub const fn preload(self) -> Self {
        Self {
            preload: true,
            ..self
        }
    }

    /// Convert the builder into a generic asset
    pub const fn into_asset_options(self) -> GenericAssetOptions {
        GenericAssetOptions::Js(self)
    }
}
