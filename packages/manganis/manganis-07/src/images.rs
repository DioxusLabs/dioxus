use const_serialize_07::SerializeConst;

use crate::{AssetOptions, AssetOptionsBuilder, AssetVariant};

/// The type of an image. You can read more about the tradeoffs between image formats [here](https://developer.mozilla.org/en-US/docs/Web/Media/Formats/Image_types)
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
#[repr(u8)]
pub enum ImageFormat {
    /// A png image. Png images cannot contain transparency and tend to compress worse than other formats
    Png,
    /// A jpg image. Jpg images can contain transparency and tend to compress better than png images
    Jpg,
    /// A webp image. Webp images can contain transparency and tend to compress better than jpg images
    Webp,
    /// An avif image. Avif images can compress slightly better than webp images but are not supported by all browsers
    Avif,
    /// An unknown image type
    Unknown,
}

/// The size of an image asset
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

/// Options for an image asset
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
pub struct ImageAssetOptions {
    ty: ImageFormat,
    low_quality_preview: bool,
    size: ImageSize,
    preload: bool,
}

impl Default for ImageAssetOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl ImageAssetOptions {
    /// Create a new builder for image asset options
    pub const fn new() -> AssetOptionsBuilder<ImageAssetOptions> {
        AssetOptions::image()
    }

    /// Create a default image asset options
    pub const fn default() -> Self {
        Self {
            ty: ImageFormat::Unknown,
            low_quality_preview: false,
            size: ImageSize::Automatic,
            preload: false,
        }
    }

    /// Check if the asset is preloaded
    pub const fn preloaded(&self) -> bool {
        self.preload
    }

    /// Get the format of the image
    pub const fn format(&self) -> ImageFormat {
        self.ty
    }

    /// Get the size of the image
    pub const fn size(&self) -> ImageSize {
        self.size
    }

    pub(crate) const fn extension(&self) -> Option<&'static str> {
        match self.ty {
            ImageFormat::Png => Some("png"),
            ImageFormat::Jpg => Some("jpg"),
            ImageFormat::Webp => Some("webp"),
            ImageFormat::Avif => Some("avif"),
            ImageFormat::Unknown => None,
        }
    }
}

impl AssetOptions {
    /// Create a new image asset builder
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/assets/image.png", AssetOptions::image());
    /// ```
    pub const fn image() -> AssetOptionsBuilder<ImageAssetOptions> {
        AssetOptionsBuilder::variant(ImageAssetOptions::default())
    }
}

impl AssetOptionsBuilder<ImageAssetOptions> {
    /// Make the asset preloaded
    ///
    /// Preloading an image will make the image start to load as soon as possible. This is useful for images that will be displayed soon after the page loads or images that may not be visible immediately, but should start loading sooner
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/assets/image.png", AssetOptions::image().with_preload(true));
    /// ```
    pub const fn with_preload(mut self, preload: bool) -> Self {
        self.variant.preload = preload;
        self
    }

    /// Sets the format of the image
    ///
    /// Choosing the right format can make your site load much faster. Webp and avif images tend to be a good default for most images
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions, ImageFormat};
    /// const _: Asset = asset!("/assets/image.png", AssetOptions::image().with_format(ImageFormat::Webp));
    /// ```
    pub const fn with_format(mut self, format: ImageFormat) -> Self {
        self.variant.ty = format;
        self
    }

    /// Sets the format of the image to [`ImageFormat::Avif`]
    ///
    /// Avif images tend to be a good default for most images rendered in browser because
    /// they compress images well
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions, ImageFormat};
    /// const _: Asset = asset!("/assets/image.png", AssetOptions::image().with_avif());
    /// ```
    pub const fn with_avif(self) -> Self {
        self.with_format(ImageFormat::Avif)
    }

    /// Sets the format of the image to [`ImageFormat::Webp`]
    ///
    /// Webp images tend to be a good default for most images rendered in browser because
    /// they compress images well
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions, ImageFormat};
    /// const _: Asset = asset!("/assets/image.png", AssetOptions::image().with_webp());
    /// ```
    pub const fn with_webp(self) -> Self {
        self.with_format(ImageFormat::Webp)
    }

    /// Sets the format of the image to [`ImageFormat::Jpg`]
    ///
    /// Jpeg images compress much better than [`ImageFormat::Png`], but worse than [`ImageFormat::Webp`] or [`ImageFormat::Avif`]
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions, ImageFormat};
    /// const _: Asset = asset!("/assets/image.png", AssetOptions::image().with_jpg());
    /// ```
    pub const fn with_jpg(self) -> Self {
        self.with_format(ImageFormat::Jpg)
    }

    /// Sets the format of the image to [`ImageFormat::Png`]
    ///
    /// Png images don't compress very well, so they are not recommended for large images
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions, ImageFormat};
    /// const _: Asset = asset!("/assets/image.png", AssetOptions::image().with_png());
    /// ```
    pub const fn with_png(self) -> Self {
        self.with_format(ImageFormat::Png)
    }

    /// Sets the size of the image
    ///
    /// If you only use the image in one place, you can set the size of the image to the size it will be displayed at. This will make the image load faster
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions, ImageSize};
    /// const _: Asset = asset!("/assets/image.png", AssetOptions::image().with_size(ImageSize::Manual { width: 512, height: 512 }));
    /// ```
    pub const fn with_size(mut self, size: ImageSize) -> Self {
        self.variant.size = size;
        self
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: self.add_hash,
            variant: AssetVariant::Image(self.variant),
        }
    }
}
