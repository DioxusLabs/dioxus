#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

#[doc(hidden)]
pub mod macro_helpers;
pub use manganis_macro::asset;
pub use manganis_macro::css_module;
pub use manganis_macro::option_asset;

pub use manganis_core::{
    Asset, AssetOptions, AssetVariant, BundledAsset, CssAssetOptions, CssModuleAssetOptions,
    FolderAssetOptions, ImageAssetOptions, ImageFormat, ImageSize, JsAssetOptions,
};
