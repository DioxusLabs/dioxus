use const_serialize::SerializeConst;

use crate::GenericAssetOptions;

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
#[derive(SerializeConst)]
pub struct ImageAssetOptions {
    ty: ImageType,
    low_quality_preview: bool,
    size: ImageSize,
    preload: bool,
}

impl ImageAssetOptions {
    /// Make the asset preloaded
    ///
    /// Preloading an image will make the image start to load as soon as possible. This is useful for images that will be displayed soon after the page loads or images that may not be visible immediately, but should start loading sooner
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

    /// Sets the format of the image
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
    /// If you only use the image in one place, you can set the size of the image to the size it will be displayed at. This will make the image load faster
    ///
    /// ```rust
    /// const _: manganis::ImageAsset = manganis::mg!(image("https://avatars.githubusercontent.com/u/79236386?s=48&v=4").size(512, 512));
    /// ```
    #[allow(unused)]
    pub const fn with_size(self, size: ImageSize) -> Self {
        Self { size, ..self }
    }

    // LQIP is currently disabled until we have the CLI set up to inject the low quality image preview after the crate is built through the linker
    // /// Make the image use a low quality preview
    // ///
    // /// A low quality preview is a small version of the image that will load faster. This is useful for large images on mobile devices that may take longer to load
    // ///
    // /// ```rust
    // /// const _: manganis::ImageAsset = manganis::mg!(image("https://avatars.githubusercontent.com/u/79236386?s=48&v=4").with_low_quality_image_preview());
    // /// ```
    // #[allow(unused)]
    // pub const fn with_low_quality_image_preview(self, low_quality_preview: bool) -> Self {
    //     Self {
    //         low_quality_preview,
    //         ..self
    //     }
    // }

    /// Convert the builder into a generic asset
    pub const fn into_asset_options(self) -> GenericAssetOptions {
        GenericAssetOptions::Image(self)
    }
}
