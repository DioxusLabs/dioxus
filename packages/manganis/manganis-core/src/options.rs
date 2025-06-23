use const_serialize::SerializeConst;

use crate::{
    CssAssetOptions, CssModuleAssetOptions, FolderAssetOptions, ImageAssetOptions, JsAssetOptions,
};

/// Settings for a generic asset
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
#[non_exhaustive]
pub struct AssetOptions {
    /// If a hash should be added to the asset path
    add_hash: bool,
    /// The variant of the asset
    variant: AssetVariant,
}

impl AssetOptions {
    /// Create a new asset options with the given variant
    pub const fn new(variant: AssetVariant) -> Self {
        Self {
            add_hash: true,
            variant,
        }
    }

    /// Set whether a hash should be added to the asset path
    pub const fn with_hash_suffix(mut self, add_hash: bool) -> Self {
        self.add_hash = add_hash;
        self
    }

    /// Get the variant of the asset
    pub const fn variant(&self) -> &AssetVariant {
        &self.variant
    }

    /// Check if a hash should be added to the asset path
    pub const fn hash_suffix(&self) -> bool {
        self.add_hash
    }

    /// Try to get the extension for the asset. If the asset options don't define an extension, this will return None
    pub const fn extension(&self) -> Option<&'static str> {
        match self.variant {
            AssetVariant::Image(image) => image.extension(),
            AssetVariant::Css(_) => Some("css"),
            AssetVariant::CssModule(_) => Some("css"),
            AssetVariant::Js(_) => Some("js"),
            AssetVariant::Folder(_) => None,
            AssetVariant::Unknown => None,
        }
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        self
    }
}

/// Settings for a specific type of asset
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
#[non_exhaustive]
pub enum AssetVariant {
    /// An image asset
    Image(ImageAssetOptions),
    /// A folder asset
    Folder(FolderAssetOptions),
    /// A css asset
    Css(CssAssetOptions),
    /// A css module asset
    CssModule(CssModuleAssetOptions),
    /// A javascript asset
    Js(JsAssetOptions),
    /// An unknown asset
    Unknown,
}

impl AssetVariant {
    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions::new(self)
    }
}
