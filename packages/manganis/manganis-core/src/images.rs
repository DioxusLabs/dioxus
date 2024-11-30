use const_serialize::SerializeConst;

use crate::AssetOptions;

/// The type of an image. You can read more about the tradeoffs between image formats [here](https://developer.mozilla.org/en-US/docs/Web/Media/Formats/Image_types)
#[derive(
    Debug,
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
        Self::new()
    }
}

impl ImageAssetOptions {
    /// Create a new image asset options
    pub const fn new() -> Self {
        Self {
            ty: ImageFormat::Unknown,
            low_quality_preview: false,
            size: ImageSize::Automatic,
            preload: false,
        }
    }

    /// Make the asset preloaded
    ///
    /// Preloading an image will make the image start to load as soon as possible. This is useful for images that will be displayed soon after the page loads or images that may not be visible immediately, but should start loading sooner
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, ImageAssetOptions};
    /// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_preload(true));
    /// ```
    pub const fn with_preload(self, preload: bool) -> Self {
        Self { preload, ..self }
    }

    /// Check if the asset is preloaded
    pub const fn preloaded(&self) -> bool {
        self.preload
    }

    /// Sets the format of the image
    ///
    /// Choosing the right format can make your site load much faster. Webp and avif images tend to be a good default for most images
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, ImageAssetOptions, ImageFormat};
    /// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_format(ImageFormat::Webp));
    /// ```
    pub const fn with_format(self, format: ImageFormat) -> Self {
        Self { ty: format, ..self }
    }

    /// Sets the format of the image to [`ImageFormat::Avif`]
    ///
    /// Avif images tend to be a good default for most images rendered in browser because
    /// they compress images well
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, ImageAssetOptions, ImageFormat};
    /// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_avif());
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
    /// # use manganis::{asset, Asset, ImageAssetOptions, ImageFormat};
    /// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_webp());
    /// ```
    pub const fn with_webp(self) -> Self {
        self.with_format(ImageFormat::Webp)
    }

    /// Sets the format of the image to [`ImageFormat::Jpg`]
    ///
    /// Jpeg images compress much better than [`ImageFormat::Png`], but worse than [`ImageFormat::Webp`] or [`ImageFormat::Avif`]
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, ImageAssetOptions, ImageFormat};
    /// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_jpg());
    /// ```
    pub const fn with_jpg(self) -> Self {
        self.with_format(ImageFormat::Jpg)
    }

    /// Sets the format of the image to [`ImageFormat::Png`]
    ///
    /// Png images don't compress very well, so they are not recommended for large images
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, ImageAssetOptions, ImageFormat};
    /// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_png());
    /// ```
    pub const fn with_png(self) -> Self {
        self.with_format(ImageFormat::Png)
    }

    /// Get the format of the image
    pub const fn format(&self) -> ImageFormat {
        self.ty
    }

    /// Sets the size of the image
    ///
    /// If you only use the image in one place, you can set the size of the image to the size it will be displayed at. This will make the image load faster
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, ImageAssetOptions, ImageSize};
    /// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_size(ImageSize::Manual { width: 512, height: 512 }));
    /// ```
    pub const fn with_size(self, size: ImageSize) -> Self {
        Self { size, ..self }
    }

    /// Get the size of the image
    pub const fn size(&self) -> ImageSize {
        self.size
    }

    // LQIP is currently disabled until we have the CLI set up to inject the low quality image preview after the crate is built through the linker
    // /// Make the image use a low quality preview
    // ///
    // /// A low quality preview is a small version of the image that will load faster. This is useful for large images on mobile devices that may take longer to load
    // ///
    // /// ```rust
    // /// # use manganis::{asset, Asset, ImageAssetOptions};
    // /// const _: Asset = manganis::asset!("/assets/image.png", ImageAssetOptions::new().with_low_quality_image_preview());
    // /// ```
    //
    // pub const fn with_low_quality_image_preview(self, low_quality_preview: bool) -> Self {
    //     Self {
    //         low_quality_preview,
    //         ..self
    //     }
    // }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions::Image(self)
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
