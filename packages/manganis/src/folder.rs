use const_serialize::SerializeConst;

use crate::GenericAssetOptions;

/// The builder for [`FolderAsset`]
#[derive(SerializeConst)]
pub struct FolderAssetOptions {}

impl FolderAssetOptions {
    /// Create a new folder asset using the builder
    pub const fn new() -> Self {
        Self {}
    }

    /// Convert the builder into a generic asset
    pub const fn into_asset_options(self) -> GenericAssetOptions {
        GenericAssetOptions::Folder(self)
    }
}
