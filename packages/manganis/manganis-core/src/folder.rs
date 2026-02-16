use const_serialize_07 as const_serialize;
use const_serialize_08::SerializeConst;

use crate::{AssetOptions, AssetOptionsBuilder};

/// The builder for a folder asset.
#[derive(
    Debug,
    Eq,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    SerializeConst,
    const_serialize::SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
#[const_serialize(crate = const_serialize_08)]
pub struct FolderAssetOptions {}

impl Default for FolderAssetOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl FolderAssetOptions {
    /// Create a new folder asset builder
    pub const fn new() -> AssetOptionsBuilder<FolderAssetOptions> {
        AssetOptions::folder()
    }

    /// Create a default folder asset options
    pub const fn default() -> Self {
        Self {}
    }
}

impl AssetOptions {
    /// Create a new folder asset builder
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/assets", AssetOptions::folder());
    /// ```
    pub const fn folder() -> AssetOptionsBuilder<FolderAssetOptions> {
        AssetOptionsBuilder::variant(FolderAssetOptions::default())
    }
}

impl AssetOptionsBuilder<FolderAssetOptions> {
    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: false,
            variant: crate::AssetVariant::Folder(self.variant),
        }
    }
}
