/// A builder for a css asset. This must be used in the [`asset!`] macro.
///
/// > **Note**: This will do nothing outside of the `asset!` macro
pub struct CssAssetBuilder;

impl CssAssetBuilder {
    /// Sets whether the css should be minified (default: true)
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// Minifying the css can make your site load faster by loading less data
    ///
    /// ```rust
    /// const _: &str = manganis::asset!(css("https://sindresorhus.com/github-markdown-css/github-markdown.css").minify(false));
    /// ```
    #[allow(unused)]
    pub const fn minify(self, minify: bool) -> Self {
        Self
    }

    /// Make the css preloaded
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// Preloading css will make the css start to load as soon as possible. This is useful for css that will be displayed soon after the page loads or css that may not be visible immediately, but should start loading sooner
    ///
    /// ```rust
    /// const _: &str = manganis::asset!(css("https://sindresorhus.com/github-markdown-css/github-markdown.css").preload());
    /// ```
    #[allow(unused)]
    pub const fn preload(self) -> Self {
        Self
    }

    /// Make the css URL encoded
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// URL encoding an image inlines the data of the css into the URL. This is useful for small css files that should load as soon as the html is parsed
    ///
    /// ```rust
    /// const _: &str = manganis::asset!(css("https://sindresorhus.com/github-markdown-css/github-markdown.css").url_encoded());
    /// ```
    #[allow(unused)]
    pub const fn url_encoded(self) -> Self {
        Self
    }
}

/// Create an css asset from the local path or url to the css
///
/// > **Note**: This will do nothing outside of the `asset!` macro
///
/// You can collect css which will be automatically minified with the css builder:
/// ```rust
/// const _: &str = manganis::asset!(css("https://sindresorhus.com/github-markdown-css/github-markdown.css"));
/// ```
/// You can mark css as preloaded to make them load faster in your app:
/// ```rust
/// const _: &str = manganis::asset!(css("https://sindresorhus.com/github-markdown-css/github-markdown.css").preload());
/// ```
#[allow(unused)]
pub const fn css(path: &'static str) -> CssAssetBuilder {
    CssAssetBuilder
}
