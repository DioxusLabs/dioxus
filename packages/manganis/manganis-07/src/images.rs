use const_serialize_07::SerializeConst;

use crate::{AssetOptions, AssetOptionsBuilder, AssetVariant};

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
    Png,
    Jpg,
    Webp,
    Avif,
    Unknown,
}

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
    Manual {
        width: u32,
        height: u32,
    },
    Automatic,
}

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
    pub const fn new() -> AssetOptionsBuilder<ImageAssetOptions> {
        AssetOptions::image()
    }

    pub const fn default() -> Self {
        Self {
            ty: ImageFormat::Unknown,
            low_quality_preview: false,
            size: ImageSize::Automatic,
            preload: false,
        }
    }

    pub const fn preloaded(&self) -> bool {
        self.preload
    }

    pub const fn format(&self) -> ImageFormat {
        self.ty
    }

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
    pub const fn image() -> AssetOptionsBuilder<ImageAssetOptions> {
        AssetOptionsBuilder::variant(ImageAssetOptions::default())
    }
}

impl AssetOptionsBuilder<ImageAssetOptions> {
    pub const fn with_preload(mut self, preload: bool) -> Self {
        self.variant.preload = preload;
        self
    }

    pub const fn with_format(mut self, format: ImageFormat) -> Self {
        self.variant.ty = format;
        self
    }

    pub const fn with_avif(self) -> Self {
        self.with_format(ImageFormat::Avif)
    }

    pub const fn with_webp(self) -> Self {
        self.with_format(ImageFormat::Webp)
    }

    pub const fn with_jpg(self) -> Self {
        self.with_format(ImageFormat::Jpg)
    }

    pub const fn with_png(self) -> Self {
        self.with_format(ImageFormat::Png)
    }

    pub const fn with_size(mut self, size: ImageSize) -> Self {
        self.variant.size = size;
        self
    }

    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: self.add_hash,
            variant: AssetVariant::Image(self.variant),
        }
    }
}
