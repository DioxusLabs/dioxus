use const_serialize::SerializeConst;

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
    ///
    /// The folder is bundled to a directory that the rest of the app joins paths onto, so it keeps
    /// the name of the source folder and [`AssetOptionsBuilder::with_hash_suffix`] is reserved for
    /// the variants that are bundled to a single file:
    ///
    /// ```rust,compile_fail,E0599
    /// # use manganis::AssetOptions;
    /// let _ = AssetOptions::folder().with_hash_suffix(false);
    /// ```
    pub const fn folder() -> AssetOptionsBuilder<FolderAssetOptions> {
        AssetOptionsBuilder::variant(FolderAssetOptions::default())
    }
}

impl AssetOptionsBuilder<FolderAssetOptions> {
    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            // A folder keeps the name of its source, see `Hashable`
            add_hash: false,
            variant: crate::AssetVariant::Folder(self.variant),
        }
    }
}
