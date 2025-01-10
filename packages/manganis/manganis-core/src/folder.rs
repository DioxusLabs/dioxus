use crate::AssetOptions;
use const_serialize::SerializeConst;

// TODO, 0.7:
// - Mark `FolderAssetOptions` as non-exhaustive.
// - Remove `FolderOptions` trait.
// - Migrate `PreservedFolderAssetOptions` to `optimize_files` field on `FolderAssetOptions`

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

    /// Convert this folder asset into a preserved folder asset.
    ///
    /// This is planned to be removed in 0.7.
    #[doc(hidden)]
    pub const fn into_preserved(self) -> PreservedFolderAssetOptions {
        PreservedFolderAssetOptions::new()
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions::Folder(self)
    }
}

/// A helper trait so crates can accept either folder type.
///
/// This is planned to be removed in 0.7.
#[doc(hidden)]
pub trait FolderOptions: Sync {
    fn optimize_files(&self) -> bool;
}

impl FolderOptions for FolderAssetOptions {
    fn optimize_files(&self) -> bool {
        true
    }
}

impl FolderOptions for PreservedFolderAssetOptions {
    fn optimize_files(&self) -> bool {
        false
    }
}

/// A preserved folder which has no optimizations.
///
/// This is planned to be removed in 0.7.
#[doc(hidden)]
#[non_exhaustive]
#[derive(
    Debug,
    Default,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct PreservedFolderAssetOptions {}

impl PreservedFolderAssetOptions {
    pub const fn new() -> Self {
        Self {}
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions::PreservedFolder(self)
    }
}
