#![doc = include_str!("../../../README.md")]
#![deny(missing_docs)]

#[cfg(feature = "macro")]
pub use manganis_macro::*;

// mod asset;
// pub use asset::*;

// mod csss;
// pub use csss::*;

// mod files;
// pub use files::*;

// mod folder;
// pub use folder::*;

// mod fonts;
// pub use fonts::*;

// mod images;
// pub use images::*;

// mod jsons;
// pub use jsons::*;

// mod jss;
// pub use jss::*;

mod builder;
pub use builder::*;

/// A trait for something that can be used in the `asset!` macro
///
/// > **Note**: These types will do nothing outside of the `asset!` macro
pub trait ForMgMacro: __private::Sealed + Sync + Send {}

mod __private {
    use super::*;

    pub trait Sealed {}

    // impl Sealed for FolderAssetBuilder {}
    // impl Sealed for FontAssetBuilder {}
    // impl Sealed for ImageAssetBuilder {}
    // impl Sealed for JsAssetBuilder {}
    // impl Sealed for JsonAssetBuilder {}
    // impl Sealed for CssAssetBuilder {}
    impl Sealed for &'static str {}
}

// impl ForMgMacro for FolderAssetBuilder {}
// impl ForMgMacro for ImageAssetBuilder {}
// impl ForMgMacro for FontAssetBuilder {}
// impl ForMgMacro for &'static str {}
