use const_serialize::SerializeConst;
use dioxus_core_types::DioxusFormattable;

use crate::{Asset, AssetBuilder};

/// An image asset that is built by the [`asset!`] macro
#[derive(Debug, PartialEq, PartialOrd, Clone, Hash, Copy)]
pub struct ImageAsset {
    /// The path to the image
    asset: Asset,
    /// A low quality preview of the image that is URL encoded
    preview: Option<&'static str>,
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
            format: None,
        }
    }

    /// Returns the path to the image
    pub const fn path(&self) -> &'static str {
        self.asset.bundled
    }

    /// Returns the url encoded preview of the image
    pub const fn preview(&self) -> Option<&'static str> {
        self.preview
    }

    /// Sets the preview of the image
    pub const fn with_preview(self, preview: Option<&'static str>) -> Self {
        Self { preview, ..self }
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
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash, SerializeConst)]
#[repr(u8)]
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

/// The size of an image asset
#[derive(SerializeConst)]
#[repr(C, u8)]
pub enum ImageSize {
    /// A manual size in pixels
    Manual {
        /// The width of the image in pixels
        width: u32,
        /// The height of the image in pixels
        height: u32,
    },
    /// The size will be automatically determined from the image source
    Automatic,
}

/// A builder for an image asset. This must be used in the [`mg!`] macro.
///
/// > **Note**: This will do nothing outside of the `mg!` macro
#[derive(SerializeConst)]
pub struct ImageAssetBuilder {
    asset: AssetBuilder,
    ty: ImageType,
    low_quality_preview: bool,
    size: ImageSize,
}

impl ImageAssetBuilder {
    /// Sets the format of the image
    ///
    /// > **Note**: This will do nothing outside of the `mg!` macro
    ///
    /// Choosing the right format can make your site load much faster. Webp and avif images tend to be a good default for most images
    ///
    /// ```rust
    /// const _: manganis::ImageAsset = manganis::mg!(image("https://avatars.githubusercontent.com/u/79236386?s=48&v=4").format(ImageType::Webp));
    /// ```
    #[allow(unused)]
    pub const fn with_format(self, format: ImageType) -> Self {
        Self { ty: format, ..self }
    }

    /// Sets the size of the image
    ///
    /// > **Note**: This will do nothing outside of the `mg!` macro
    ///
    /// If you only use the image in one place, you can set the size of the image to the size it will be displayed at. This will make the image load faster
    ///
    /// ```rust
    /// const _: manganis::ImageAsset = manganis::mg!(image("https://avatars.githubusercontent.com/u/79236386?s=48&v=4").size(512, 512));
    /// ```
    #[allow(unused)]
    pub const fn with_size(self, size: ImageSize) -> Self {
        Self { size, ..self }
    }

    /// Make the image use a low quality preview
    ///
    /// > **Note**: This will do nothing outside of the `mg!` macro
    ///
    /// A low quality preview is a small version of the image that will load faster. This is useful for large images on mobile devices that may take longer to load
    ///
    /// ```rust
    /// const _: manganis::ImageAsset = manganis::mg!(image("https://avatars.githubusercontent.com/u/79236386?s=48&v=4").with_low_quality_image_preview());
    /// ```
    #[allow(unused)]
    pub const fn with_low_quality_image_preview(self, low_quality_preview: bool) -> Self {
        Self {
            low_quality_preview,
            ..self
        }
    }
}

/// Create an image asset from the local path or url to the image
#[allow(unused)]
pub const fn image(path: &'static str) -> ImageAssetBuilder {
    ImageAssetBuilder {
        asset: AssetBuilder::new(path),
        ty: ImageType::Png,
        low_quality_preview: false,
        size: ImageSize::Automatic,
    }
}
