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
    /// Keep all the files original and intact.
    preserve_files: bool,
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
            preserve_files: false,
        }
    }

    /// Set whether the original files should undergo minor processing or none at all.
    #[allow(unused)]
    pub const fn with_preserve_files(self, preserve_files: bool) -> Self {
        Self { preserve_files }
    }

    /// Check if the folder's files should be fully preserved.
    pub const fn preserve_files(&self) -> bool {
        self.preserve_files
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions::Folder(self)
    }
}
