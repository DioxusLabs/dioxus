use const_serialize::SerializeConst;

use crate::GenericAssetOptions;

/// A builder for a css asset. This must be used in the [`mg!`] macro.
#[derive(SerializeConst)]
pub struct CssAssetOptions {
    minify: bool,
    preload: bool,
}

impl CssAssetOptions {
    /// Create a new css asset using the builder
    pub const fn new() -> Self {
        Self {
            preload: false,
            minify: true,
        }
    }

    /// Sets whether the css should be minified (default: true)
    ///
    /// Minifying the css can make your site load faster by loading less data
    ///
    /// ```rust
    /// const _: &str = manganis::mg!(css("https://sindresorhus.com/github-markdown-css/github-markdown.css").minify(false));
    /// ```
    #[allow(unused)]
    pub const fn minify(self, minify: bool) -> Self {
        Self { minify, ..self }
    }

    /// Make the asset preloaded
    ///
    /// Preloading css will make the image start to load as soon as possible. This is useful for css that is used soon after the page loads or css that may not be used immediately, but should start loading sooner
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
        GenericAssetOptions::Css(self)
    }
}
