use const_serialize_07::SerializeConst;

use crate::{
    CssAssetOptions, CssModuleAssetOptions, FolderAssetOptions, ImageAssetOptions, JsAssetOptions,
};

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
#[non_exhaustive]
pub struct AssetOptions {
    pub(crate) add_hash: bool,
    pub(crate) variant: AssetVariant,
}

impl AssetOptions {
    pub const fn builder() -> AssetOptionsBuilder<()> {
        AssetOptionsBuilder::new()
    }

    pub const fn variant(&self) -> &AssetVariant {
        &self.variant
    }

    pub const fn hash_suffix(&self) -> bool {
        self.add_hash
    }

    pub const fn extension(&self) -> Option<&'static str> {
        match self.variant {
            AssetVariant::Image(image) => image.extension(),
            AssetVariant::Css(_) => Some("css"),
            AssetVariant::CssModule(_) => Some("css"),
            AssetVariant::Js(_) => Some("js"),
            AssetVariant::Folder(_) => None,
            AssetVariant::Unknown => None,
        }
    }

    pub const fn into_asset_options(self) -> AssetOptions {
        self
    }
}

pub struct AssetOptionsBuilder<T> {
    pub(crate) add_hash: bool,
    pub(crate) variant: T,
}

impl Default for AssetOptionsBuilder<()> {
    fn default() -> Self {
        Self::default()
    }
}

impl AssetOptionsBuilder<()> {
    pub const fn new() -> Self {
        Self {
            add_hash: true,
            variant: (),
        }
    }

    pub const fn default() -> Self {
        Self::new()
    }

    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: self.add_hash,
            variant: AssetVariant::Unknown,
        }
    }
}

impl<T> AssetOptionsBuilder<T> {
    pub(crate) const fn variant(variant: T) -> Self {
        Self {
            add_hash: true,
            variant,
        }
    }

    pub const fn with_hash_suffix(mut self, add_hash: bool) -> Self {
        self.add_hash = add_hash;
        self
    }
}

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
#[repr(C, u8)]
#[non_exhaustive]
pub enum AssetVariant {
    Image(ImageAssetOptions),
    Folder(FolderAssetOptions),
    Css(CssAssetOptions),
    CssModule(CssModuleAssetOptions),
    Js(JsAssetOptions),
    Unknown,
}
