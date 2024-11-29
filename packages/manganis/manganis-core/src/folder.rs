use const_serialize::SerializeConst;

use crate::AssetOptions;

/// The builder for [`FolderAsset`]
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
pub struct FolderAssetOptions {}

impl Default for FolderAssetOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl FolderAssetOptions {
    /// Create a new folder asset using the builder
    pub const fn new() -> Self {
        Self {}
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions::Folder(self)
    }
}
