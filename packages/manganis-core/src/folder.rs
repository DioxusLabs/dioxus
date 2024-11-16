use const_serialize::SerializeConst;

use crate::AssetOptions;

/// The builder for [`FolderAsset`]
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash, SerializeConst)]
pub struct FolderAssetOptions {}

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
