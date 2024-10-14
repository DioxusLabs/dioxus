///
pub const fn video(self) -> VideoAsset {
    VideoAsset::new(self.src)
}

///
pub const fn json(self) -> JsonAsset {
    JsonAsset::new(self.src)
}

///
pub const fn css(self) -> CssAsset {
    CssAsset::new(self.src)
}

///
pub const fn javascript(self) -> JavaScriptAsset {
    JavaScriptAsset::new(self.src)
}

///
pub const fn typescript(self) -> TypeScriptAsset {
    TypeScriptAsset::new(self.src)
}

///
pub struct CssAsset {
    src: AssetSource,
}

impl CssAsset {
    ///
    pub const fn new(src: AssetSource) -> Self {
        Self { src }
    }

    ///
    pub const fn minify(self, minify: bool) -> Self {
        todo!()
    }
}

///
pub struct VideoAsset {
    src: AssetSource,
}

impl VideoAsset {
    ///
    pub const fn new(src: AssetSource) -> Self {
        Self { src }
    }
}

///
///
pub struct JsonAsset {
    src: AssetSource,
}
impl JsonAsset {
    ///
    pub const fn new(src: AssetSource) -> Self {
        Self { src }
    }

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
        self
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
        self
    }
}

///
///
///
pub struct JavaScriptAsset {
    src: AssetSource,
}
impl JavaScriptAsset {
    ///
    pub const fn new(src: AssetSource) -> Self {
        Self { src }
    }
}

///
///
pub struct TypeScriptAsset {
    src: AssetSource,
}

impl TypeScriptAsset {
    ///
    pub const fn new(src: AssetSource) -> Self {
        Self { src }
    }
}
///
pub struct FolderAsset {
    src: AssetSource,
}

impl FolderAsset {
    ///
    pub const fn new(src: AssetSource) -> Self {
        Self { src }
    }

    ///
    pub const fn build(self) -> FolderAsset {
        FolderAsset { src: self.src }
    }
}

/// Asset
#[derive(Debug, PartialEq, Clone, Copy, Hash)]
pub struct ImageAsset {
    src: AssetSource,
}
