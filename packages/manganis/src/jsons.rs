use crate::Asset;

/// A builder for a json asset. This must be used in the [`asset!`] macro.
///
/// > **Note**: This will do nothing outside of the `asset!` macro
pub struct JsonAssetBuilder;

impl JsonAssetBuilder {
    /// Make the json preloaded
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// Preloading json will make the json start to load as soon as possible. This is useful for json that will be run soon after the page loads or json that may not be used immediately, but should start loading sooner
    ///
    /// ```rust
    /// const _: &str = manganis::asset!(json("assets/data.json").preload());
    /// ```
    #[allow(unused)]
    pub const fn preload(self) -> Self {
        Self
    }

    /// Make the json URL encoded
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// URL encoding an image inlines the data of the json into the URL. This is useful for small json files that should load as soon as the html is parsed
    ///
    /// ```rust
    /// const _: &str = manganis::asset!(json("assets/data.json").url_encoded());
    /// ```
    #[allow(unused)]
    pub const fn url_encoded(self) -> Self {
        Self
    }
}
