use const_serialize_07 as const_serialize;
use const_serialize_08::SerializeConst;

use crate::{
    CssAssetOptions, CssModuleAssetOptions, FolderAssetOptions, ImageAssetOptions, JsAssetOptions,
};

/// Settings for a generic asset
#[derive(
    Debug,
    Eq,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    SerializeConst,
    const_serialize::SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
#[const_serialize(crate = const_serialize_08)]
#[non_exhaustive]
pub struct AssetOptions {
    /// If a hash should be added to the asset path
    pub(crate) add_hash: bool,
    /// The variant of the asset
    pub(crate) variant: AssetVariant,
}

impl AssetOptions {
    /// Create a new asset options builder
    pub const fn builder() -> AssetOptionsBuilder<()> {
        AssetOptionsBuilder::new()
    }

    /// Get the variant of the asset
    pub const fn variant(&self) -> &AssetVariant {
        &self.variant
    }

    /// Check if a hash should be added to the asset path
    pub const fn hash_suffix(&self) -> bool {
        self.add_hash
    }

    /// Try to get the extension for the asset. If the asset options don't define an extension, this will return None
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

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        self
    }
}

/// A builder for [`AssetOptions`]
///
/// ```rust
/// # use manganis::{AssetOptions, Asset, asset};
/// static ASSET: Asset = asset!(
///     "/assets/style.css",
///     AssetOptions::builder()
///     .with_hash_suffix(false)
/// );
/// ```
pub struct AssetOptionsBuilder<T> {
    /// If a hash should be added to the asset path
    pub(crate) add_hash: bool,
    /// The variant of the asset
    pub(crate) variant: T,
}

impl Default for AssetOptionsBuilder<()> {
    fn default() -> Self {
        Self::default()
    }
}

impl AssetOptionsBuilder<()> {
    /// Create a new asset options builder with an unknown variant
    pub const fn new() -> Self {
        Self {
            add_hash: true,
            variant: (),
        }
    }

    /// Create a default asset options builder
    pub const fn default() -> Self {
        Self::new()
    }

    /// Convert the builder into asset options with the given variant
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: self.add_hash,
            variant: AssetVariant::Unknown,
        }
    }
}

impl<T> AssetOptionsBuilder<T> {
    /// Create a new asset options builder with the given variant
    pub const fn variant(variant: T) -> Self {
        Self {
            add_hash: true,
            variant,
        }
    }

    /// Set whether a hash should be added to the asset path. Manganis adds hashes to asset paths by default
    /// for [cache busting](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Caching#cache_busting).
    /// With hashed assets, you can serve the asset with a long expiration time, and when the asset changes,
    /// the hash in the path will change, causing the browser to fetch the new version.
    ///
    /// This method will only effect if the hash is added to the bundled asset path. If you are using the asset
    /// macro, the asset struct still needs to be used in your rust code to ensure the asset is included in the binary.
    ///
    /// <div class="warning">
    ///
    /// If you are using an asset outside of rust code where you know what the asset hash will be, you must use the
    /// `#[used]` attribute to ensure the asset is included in the binary even if it is not referenced in the code.
    ///
    /// ```rust
    /// #[used]
    /// static ASSET: manganis::Asset = manganis::asset!(
    ///     "/assets/style.css",
    ///     manganis::AssetOptions::builder()
    ///         .with_hash_suffix(false)
    /// );
    /// ```
    ///
    /// </div>
    pub const fn with_hash_suffix(mut self, add_hash: bool) -> Self {
        self.add_hash = add_hash;
        self
    }
}

/// Settings for a specific type of asset
#[derive(
    Debug,
    Eq,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    SerializeConst,
    const_serialize::SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
#[const_serialize(crate = const_serialize_08)]
#[repr(C, u8)]
#[non_exhaustive]
pub enum AssetVariant {
    /// An image asset
    Image(ImageAssetOptions),
    /// A folder asset
    Folder(FolderAssetOptions),
    /// A css asset
    Css(CssAssetOptions),
    /// A css module asset
    CssModule(CssModuleAssetOptions),
    /// A javascript asset
    Js(JsAssetOptions),
    /// An unknown asset
    Unknown,
}
