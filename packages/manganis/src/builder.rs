use std::path::{Path, PathBuf};

/// Asset
#[derive(Debug, PartialEq, Clone, Copy, Hash)]
pub struct Asset {
    src: AssetSource,
}

impl From<Asset> for String {
    fn from(value: Asset) -> Self {
        value.to_string()
    }
}
impl From<Asset> for Option<String> {
    fn from(value: Asset) -> Self {
        Some(value.to_string())
    }
}

impl Asset {
    ///
    pub const fn new(src: AssetSource) -> Self {
        Self { src }
    }

    ///
    pub const fn folder(self) -> FolderAsset {
        FolderAsset::new(self.src)
    }

    /// Convert this asset into an image asset
    pub const fn image(self) -> ImageAsset {
        ImageAsset::new(self.src)
    }

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

    /// Get the path to the asset
    pub fn path(&self) -> PathBuf {
        PathBuf::from(self.src.input.to_string())
    }

    /// Get the path to the asset
    pub fn relative_path(&self) -> PathBuf {
        PathBuf::from(self.src.input.trim_start_matches("/").to_string())
    }
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.src.resolve().display())
    }
}

///
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash)]
pub struct AssetSource {
    /// The input URI given to the macro
    pub input: &'static str,

    /// The sourcefile of the asset
    pub source_file: &'static str,

    /// The absolute path to the asset on the filesystem
    pub local: &'static str,

    ///
    pub bundled: &'static str,
}

impl AssetSource {
    /// Return a canonicalized path to the asset
    pub fn resolve(&self) -> PathBuf {
        // if we're running with cargo in the loop, we can use the absolute path.
        // this is non-bundled situations
        if let Ok(_manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            return PathBuf::from(self.local);
        }

        // Otherwise, we need to resolve the bundled path against the basepath.
        // on native this will be the bundled path
        base_path()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join(self.input.trim_start_matches('/'))
    }
}

fn base_path() -> Option<PathBuf> {
    // Use the prescence of the bundle to determine if we're in dev mode
    // todo: for other platforms, we should check their bundles too. This currently only works for macOS and iOS
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        // Note that this will return `target/debug` if you're in debug mode - not reliable check if we're in dev mode
        if let Some(resources) = core_foundation::bundle::CFBundle::main_bundle().resources_path() {
            return dunce::canonicalize(resources).ok();
        }
    }

    // todo: this needs to be real canonicalizations
    // let root;

    // #[cfg(target_os = "wasm32-unknown-unknown")]
    // {
    //     root = "/".to_string();
    // }

    // #[cfg(not(target_os = "wasm32-unknown-unknown"))]
    // {
    //     root = std::env::current_dir().unwrap();
    // }

    // let var_base_path = std::env::var("MANGANIS_BASE_PATH")
    //     .unwrap_or_else(|| "/".to_string())
    //     .map(|p| PathBuf::from(p));

    // var_base_path.unwrap_or(root)
    None
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

impl std::fmt::Display for ImageAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.src.resolve().display())
    }
}

/// The type of an image. You can read more about the tradeoffs between image formats [here](https://developer.mozilla.org/en-US/docs/Web/Media/Formats/Image_types)
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash)]
pub enum ImageType {
    /// A png image. Png images cannot contain transparency and tend to compress worse than other formats
    Png,
    /// A jpg image. Jpg images can contain transparency and tend to compress better than png images
    Jpg,
    /// A webp image. Webp images can contain transparency and tend to compress better than jpg images
    Webp,
    /// An avif image. Avif images can compress slightly better than webp images but are not supported by all browsers
    Avif,
}

impl ImageAsset {
    ///
    pub const fn new(src: AssetSource) -> Self {
        Self { src }
    }

    ///
    pub const fn build(self) -> ImageAsset {
        ImageAsset { src: self.src }
    }

    /// Sets the format of the image
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// Choosing the right format can make your site load much faster. Webp and avif images tend to be a good default for most images
    ///
    /// ```rust
    /// const _: manganis::ImageAsset = manganis::asset!(image("https://avatars.githubusercontent.com/u/79236386?s=48&v=4").format(ImageType::Webp));
    /// ```
    #[allow(unused)]
    pub const fn format(self, format: ImageType) -> Self {
        self
    }

    /// Sets the size of the image
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// If you only use the image in one place, you can set the size of the image to the size it will be displayed at. This will make the image load faster
    ///
    /// ```rust
    /// const _: manganis::ImageAsset = manganis::asset!(image("https://avatars.githubusercontent.com/u/79236386?s=48&v=4").size(512, 512));
    /// ```
    #[allow(unused)]
    pub const fn size(self, x: u32, y: u32) -> Self {
        self
    }

    /// Make the image use a low quality preview
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// A low quality preview is a small version of the image that will load faster. This is useful for large images on mobile devices that may take longer to load
    ///
    /// ```rust
    /// const _: manganis::ImageAsset = manganis::asset!(image("https://avatars.githubusercontent.com/u/79236386?s=48&v=4").low_quality_preview());
    /// ```
    #[allow(unused)]
    pub const fn low_quality_preview(self) -> Self {
        self
    }

    /// Make the image preloaded
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// Preloading an image will make the image start to load as soon as possible. This is useful for images that will be displayed soon after the page loads or images that may not be visible immediately, but should start loading sooner
    ///
    /// ```rust
    /// const _: manganis::ImageAsset = manganis::asset!(image("https://avatars.githubusercontent.com/u/79236386?s=48&v=4").preload());
    /// ```
    #[allow(unused)]
    pub const fn preload(self) -> Self {
        self
    }

    /// Make the image URL encoded
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// URL encoding an image inlines the data of the image into the URL. This is useful for small images that should load as soon as the html is parsed
    ///
    /// ```rust
    /// const _: manganis::ImageAsset = manganis::asset!(image("https://avatars.githubusercontent.com/u/79236386?s=48&v=4").url_encoded());
    /// ```
    #[allow(unused)]
    pub const fn url_encoded(self) -> Self {
        self
    }
}
