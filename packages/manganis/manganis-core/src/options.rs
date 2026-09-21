use const_serialize::SerializeConst;

use crate::{
    CssAssetOptions, CssModuleAssetOptions, FolderAssetOptions, ImageAssetOptions, JsAssetOptions,
};

/// A hint to the browser about how a preloaded asset should be prioritized against the other
/// resources the page fetches. This maps to the
/// [`fetchpriority`](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Attributes/fetchpriority)
/// attribute of the `<link rel="preload">` tag the CLI writes into `index.html`.
///
/// ```rust
/// # use manganis::{asset, Asset, AssetOptions, FetchPriority};
/// const _: Asset = asset!(
///     "/assets/style.css",
///     AssetOptions::css()
///         .with_preload(true)
///         .with_fetch_priority(FetchPriority::High)
/// );
/// ```
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
#[repr(u8)]
pub enum FetchPriority {
    /// Let the browser choose the priority based on the type of the resource. The
    /// `fetchpriority` attribute is left off the generated tag.
    Auto,
    /// Fetch the asset at a higher priority than other resources of the same type.
    High,
    /// Fetch the asset at a lower priority than other resources of the same type.
    Low,
}

impl Default for FetchPriority {
    fn default() -> Self {
        Self::default()
    }
}

impl FetchPriority {
    /// Create the default fetch priority, [`FetchPriority::Auto`]
    pub const fn default() -> Self {
        Self::Auto
    }

    /// Get the value of the `fetchpriority` attribute for this priority, or `None` if the
    /// attribute should be left off the tag entirely.
    pub const fn attribute_value(&self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::High => Some("high"),
            Self::Low => Some("low"),
        }
    }
}

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
    serde::Serialize,
    serde::Deserialize,
)]
#[non_exhaustive]
pub struct AssetOptions {
    /// If a hash should be added to the asset path
    pub(crate) add_hash: bool,
    /// The variant of the asset
    pub(crate) variant: AssetVariant,
    /// The priority hint for the preload tag of the asset
    pub(crate) fetch_priority: FetchPriority,
    /// The position of the preload tag of the asset relative to the other preloaded assets
    pub(crate) preload_order: i32,
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

    /// Get the priority hint the browser should use when fetching the preloaded asset
    pub const fn fetch_priority(&self) -> FetchPriority {
        self.fetch_priority
    }

    /// Get the position of the preload tag of the asset relative to the other preloaded assets.
    /// Preload tags are written in ascending order of this value.
    pub const fn preload_order(&self) -> i32 {
        self.preload_order
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
    /// The priority hint for the preload tag of the asset
    pub(crate) fetch_priority: FetchPriority,
    /// The position of the preload tag of the asset relative to the other preloaded assets
    pub(crate) preload_order: i32,
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
            fetch_priority: FetchPriority::default(),
            preload_order: 0,
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
            fetch_priority: self.fetch_priority,
            preload_order: self.preload_order,
        }
    }
}

impl<T> AssetOptionsBuilder<T> {
    /// Create a new asset options builder with the given variant
    pub const fn variant(variant: T) -> Self {
        Self {
            add_hash: true,
            variant,
            fetch_priority: FetchPriority::default(),
            preload_order: 0,
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

    /// Set the priority hint the browser should use when fetching the asset (default:
    /// [`FetchPriority::Auto`]).
    ///
    /// This only has an effect on assets that are preloaded with `with_preload(true)`; it sets the
    /// `fetchpriority` attribute of the generated `<link rel="preload">` tag. Raising the priority
    /// of an asset the first render depends on, such as a font or a hero image, lets the browser
    /// fetch it ahead of resources it would otherwise treat as equally important.
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions, FetchPriority};
    /// const _: Asset = asset!(
    ///     "/assets/image.png",
    ///     AssetOptions::image()
    ///         .with_preload(true)
    ///         .with_fetch_priority(FetchPriority::High)
    /// );
    /// ```
    pub const fn with_fetch_priority(mut self, fetch_priority: FetchPriority) -> Self {
        self.fetch_priority = fetch_priority;
        self
    }

    /// Set the position of the asset's preload tag relative to the other preloaded assets
    /// (default: `0`).
    ///
    /// Preload tags are written into the head in ascending order of this value, so a negative
    /// order pulls an asset in front of every asset that uses the default. Browsers fetch
    /// resources of the same computed priority in the order they discover them, so this is the
    /// knob to reach for when several assets share one [`FetchPriority`]. Assets with an equal
    /// order are written in a stable, alphabetical order.
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!(
    ///     "/assets/style.css",
    ///     AssetOptions::css()
    ///         .with_preload(true)
    ///         .with_preload_order(-1)
    /// );
    /// ```
    pub const fn with_preload_order(mut self, preload_order: i32) -> Self {
        self.preload_order = preload_order;
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
    serde::Serialize,
    serde::Deserialize,
)]
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
