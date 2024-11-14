use const_serialize::{ConstStr, SerializeConst};

use crate::{CssAssetOptions, FolderAssetOptions, ImageAssetOptions, JsAssetOptions};

/// Settings for a generic asset
#[derive(Debug, SerializeConst)]
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
    pub(crate) const fn extension(&self) -> Option<&'static str> {
        match self {
            AssetOptions::Image(image) => image.extension(),
            AssetOptions::Folder(_) => None,
            AssetOptions::Css(_) => Some("css"),
            AssetOptions::Js(_) => Some("js"),
            AssetOptions::Unknown => None,
        }
    }
}

/// A builder for a generic asset. For configuration options specific to the asset type, see [`image`], [`folder`], [`css`], and [`js`]
#[derive(SerializeConst)]
pub struct AssetBuilder {
    local_path: ConstStr,
    config: AssetOptions,
}

impl AssetBuilder {
    /// Create a new asset builder
    pub const fn new(local_path: &str) -> Self {
        Self {
            local_path: ConstStr::new(local_path),
            config: AssetOptions::Unknown,
        }
    }

    /// Set the config for the asset
    pub const fn with_config(self, config: AssetOptions) -> Self {
        Self { config, ..self }
    }
}
