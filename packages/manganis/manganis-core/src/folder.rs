use crate::{AssetOptions, AssetOptionsBuilder};
use const_serialize::{ConstStr, SerializeConst};

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
        let (mount_path, has_mount_path) = match self.mount_path {
            Some(path) => (ConstStr::new(path), true),
            None => (ConstStr::new(""), false),
        };

        AssetOptions {
            add_hash: false,
            variant: crate::AssetVariant::Folder(self.variant),
            mount_path,
            has_mount_path,
        }
    }
}
