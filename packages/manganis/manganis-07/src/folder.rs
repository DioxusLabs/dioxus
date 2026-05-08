use const_serialize_07::SerializeConst;

use crate::{AssetOptions, AssetOptionsBuilder};

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
pub struct FolderAssetOptions {}

impl Default for FolderAssetOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl FolderAssetOptions {
    pub const fn new() -> AssetOptionsBuilder<FolderAssetOptions> {
        AssetOptions::folder()
    }

    pub const fn default() -> Self {
        Self {}
    }
}

impl AssetOptions {
    pub const fn folder() -> AssetOptionsBuilder<FolderAssetOptions> {
        AssetOptionsBuilder::variant(FolderAssetOptions::default())
    }
}

impl AssetOptionsBuilder<FolderAssetOptions> {
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: false,
            variant: crate::AssetVariant::Folder(self.variant),
        }
    }
}
