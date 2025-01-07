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
pub struct FolderAssetOptions {
    /// If the folder's files should be optimized.
    /// Defaults to true.
    optimize_files: bool,
}

impl Default for FolderAssetOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl FolderAssetOptions {
    /// Create a new folder asset using the builder
    pub const fn new() -> Self {
        Self {
            optimize_files: true,
        }
    }

    /// Set whether the folder's files should be optimized.
    /// Defaults to true.
    #[allow(unused)]
    pub const fn with_optimize_files(self, preserve_files: bool) -> Self {
        Self {
            optimize_files: preserve_files,
        }
    }

    /// Check if the folder's files should be optimized.
    pub const fn optimize_files(&self) -> bool {
        self.optimize_files
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions::Folder(self)
    }
}
