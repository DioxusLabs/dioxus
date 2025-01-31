#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

mod hash;
#[doc(hidden)]
pub mod macro_helpers;
pub use manganis_macro::asset;

pub use manganis_core::{
    Asset, AssetOptions, BundledAsset, CssAssetOptions, FolderAssetOptions, ImageAssetOptions,
    ImageFormat, ImageSize, JsAssetOptions,
};
