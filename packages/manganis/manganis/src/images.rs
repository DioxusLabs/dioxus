use dioxus_core_types::DioxusFormattable;

use crate::Asset;

/// An image asset that is built by the [`asset!`] macro
#[derive(Debug, PartialEq, PartialOrd, Clone, Hash, Copy)]
pub struct ImageAsset {
    /// The path to the image
    asset: Asset,
    /// A low quality preview of the image that is URL encoded
    preview: Option<&'static str>,
    /// A caption for the image
    caption: Option<&'static str>,
    /// The format of the image
    format: Option<ImageType>,
}

impl Asset {
    /// Convert this asset into an image asset
    pub const fn image(self) -> ImageAsset {
        ImageAsset::new(self)
    }
}

impl ImageAsset {
    /// Creates a new image asset
    pub const fn new(path: Asset) -> Self {
        Self {
            asset: path,
            preview: None,
            caption: None,
            format: None,
        }
    }

    /// Returns the path to the image
    pub const fn path(&self) -> &'static str {
        self.asset.bundled
    }

    /// Returns the preview of the image
    pub const fn preview(&self) -> Option<&'static str> {
        self.preview
    }

    /// Sets the preview of the image
    pub const fn with_preview(self, preview: Option<&'static str>) -> Self {
        Self { preview, ..self }
    }

    /// Returns the caption of the image
    pub const fn caption(&self) -> Option<&'static str> {
        self.caption
    }

    /// Sets the caption of the image
    pub const fn with_caption(self, caption: Option<&'static str>) -> Self {
        Self { caption, ..self }
    }

    /// Sets the format of the image
    pub const fn format(self, format: Option<ImageType>) -> Self {
        Self { format, ..self }
    }
}

impl std::ops::Deref for ImageAsset {
    type Target = Asset;

    fn deref(&self) -> &Self::Target {
        &self.asset
    }
}

impl std::fmt::Display for ImageAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.asset.fmt(f)
    }
}

impl DioxusFormattable for ImageAsset {
    fn format(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(self.to_string())
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

/// A builder for an image asset. This must be used in the [`asset!`] macro.
///
/// > **Note**: This will do nothing outside of the `asset!` macro
pub struct ImageAssetBuilder;

impl ImageAssetBuilder {
    /// Sets the format of the image
    #[allow(unused)]
    pub const fn format(self, format: ImageType) -> Self {
        Self
    }

    /// Sets the size of the image
    #[allow(unused)]
    pub const fn size(self, x: u32, y: u32) -> Self {
        Self
    }

    /// Make the image use a low quality preview
    #[allow(unused)]
    pub const fn low_quality_preview(self) -> Self {
        Self
    }

    /// Make the image preloaded
    #[allow(unused)]
    pub const fn preload(self) -> Self {
        Self
    }

    /// Make the image URL encoded
    #[allow(unused)]
    pub const fn url_encoded(self) -> Self {
        Self
    }
}

/// Create an image asset from the local path or url to the image
#[allow(unused)]
pub const fn image(path: &'static str) -> ImageAssetBuilder {
    ImageAssetBuilder
}
