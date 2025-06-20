#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

#[doc(hidden)]
pub mod macro_helpers;
pub use manganis_macro::{asset, css_module};

pub use manganis_core::{
    Asset, AssetOptions, BundledAsset, CssAssetOptions, CssModuleAssetOptions, FolderAssetOptions,
    ImageAssetOptions, ImageFormat, ImageSize, JsAssetOptions,
};
