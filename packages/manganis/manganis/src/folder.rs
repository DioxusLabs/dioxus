use crate::Asset;

/// This is basically a compile-time version of ResourceAsset
/// A struct that contains the relative and absolute paths of an asset
#[derive(Debug, PartialEq, PartialOrd, Clone, Hash)]
pub struct FolderAsset {
    src: Asset,
}

impl Asset {
    /// Create a new folder using `Asset` as the base type
    pub const fn folder(self) -> FolderAsset {
        FolderAsset::new(self)
    }
}

impl FolderAsset {
    /// Create a new folder asset from an `Asset`
    pub const fn new(src: Asset) -> Self {
        Self { src }
    }
}

/// The builder for [`FolderAsset`]
pub struct FolderAssetBuilder;

/// Create an folder asset from the local path
#[allow(unused)]
pub const fn folder(src: &'static str) -> FolderAssetBuilder {
    FolderAssetBuilder {}
}
