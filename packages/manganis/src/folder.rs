use crate::Asset;

/// This is basically a compile-time version of ResourceAsset
/// A struct that contains the relative and absolute paths of an asset
#[derive(Debug, PartialEq, PartialOrd, Clone, Hash)]
pub struct FolderAsset {
    src: Asset,
}

impl Asset {
    ///
    pub const fn folder(self) -> FolderAsset {
        FolderAsset::new(self)
    }
}

impl FolderAsset {
    ///
    pub const fn new(src: Asset) -> Self {
        Self { src }
    }
}

///
pub struct FolderAssetBuilder;

/// Create an folder asset from the local path
///
/// > **Note**: This will do nothing outside of the `asset!` macro
///
/// The folder builder collects an arbitrary local folder. Relative paths are resolved relative to the package root
/// ```rust
/// const _: &str = manganis::asset!("/assets");
/// ```
#[allow(unused)]
pub const fn folder(src: &'static str) -> FolderAssetBuilder {
    FolderAssetBuilder {}
}
