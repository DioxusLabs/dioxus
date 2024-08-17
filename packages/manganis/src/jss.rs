/// A builder for a javascript asset. This must be used in the [`asset!`] macro.
///
/// > **Note**: This will do nothing outside of the `asset!` macro
pub struct JsAssetBuilder;

impl JsAssetBuilder {
    /// Sets whether the js should be minified (default: true)
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// Minifying the js can make your site load faster by loading less data
    ///
    /// ```rust
    /// const _: &str = manganis::asset!(js("assets/script.js").minify(false));
    /// ```
    #[allow(unused)]
    pub const fn minify(self, minify: bool) -> Self {
        Self
    }

    /// Make the js preloaded
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// Preloading js will make the js start to load as soon as possible. This is useful for js that will be run soon after the page loads or js that may not be used immediately, but should start loading sooner
    ///
    /// ```rust
    /// const _: &str = manganis::asset!(js("assets/script.js").preload());
    /// ```
    #[allow(unused)]
    pub const fn preload(self) -> Self {
        Self
    }

    /// Make the js URL encoded
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// URL encoding an image inlines the data of the js into the URL. This is useful for small js files that should load as soon as the html is parsed
    ///
    /// ```rust
    /// const _: &str = manganis::asset!(js("assets/script.js").url_encoded());
    /// ```
    #[allow(unused)]
    pub const fn url_encoded(self) -> Self {
        Self
    }
}
