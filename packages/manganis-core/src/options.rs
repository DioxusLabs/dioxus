use const_serialize::SerializeConst;

use crate::{CssAssetOptions, FolderAssetOptions, ImageAssetOptions, JsAssetOptions};

/// Settings for a generic asset
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
#[repr(C, u8)]
#[non_exhaustive]
pub enum AssetOptions {
    /// An image asset
    Image(ImageAssetOptions),
    /// A folder asset
    Folder(FolderAssetOptions),
    /// A css asset
    Css(CssAssetOptions),
    /// A javascript asset
    Js(JsAssetOptions),
    /// An unknown asset
    Unknown,
}

impl AssetOptions {
    /// Try to get the extension for the asset. If the asset options don't define an extension, this will return None
    pub const fn extension(&self) -> Option<&'static str> {
        match self {
            AssetOptions::Image(image) => image.extension(),
            AssetOptions::Css(_) => Some("css"),
            AssetOptions::Js(_) => Some("js"),
            AssetOptions::Folder(_) => None,
            AssetOptions::Unknown => None,
        }
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> Self {
        self
    }
}
