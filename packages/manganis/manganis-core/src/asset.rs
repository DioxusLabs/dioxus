use crate::AssetOptions;
use const_serialize::{ConstStr, SerializeConst};
use std::path::PathBuf;

/// An asset that should be copied by the bundler with some options. This type will be
/// serialized into the binary and added to the link section [`LinkSection::CURRENT`](crate::linker::LinkSection::CURRENT).
/// CLIs that support manganis, should pull out the assets from the link section, optimize,
/// and write them to the filesystem at [`BundledAsset::bundled_path`] for the application
/// to use.
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
pub struct BundledAsset {
    /// The absolute path of the asset
    absolute_source_path: ConstStr,
    /// The bundled path of the asset
    bundled_path: ConstStr,
    /// The options for the asset
    options: AssetOptions,
}

impl BundledAsset {
    #[doc(hidden)]
    /// This should only be called from the macro
    /// Create a new asset
    pub const fn new(
        absolute_source_path: &'static str,
        bundled_path: &'static str,
        options: AssetOptions,
    ) -> Self {
        Self {
            absolute_source_path: ConstStr::new(absolute_source_path),
            bundled_path: ConstStr::new(bundled_path),
            options,
        }
    }

    #[doc(hidden)]
    /// This should only be called from the macro
    /// Create a new asset but with a relative path
    ///
    /// This method is deprecated and will be removed in a future release.
    #[deprecated(
        note = "Relative asset!() paths are not supported. Use a path like `/assets/myfile.png` instead of `./assets/myfile.png`"
    )]
    pub const fn new_relative(
        absolute_source_path: &'static str,
        bundled_path: &'static str,
        options: AssetOptions,
    ) -> Self {
        Self::new(absolute_source_path, bundled_path, options)
    }

    #[doc(hidden)]
    /// This should only be called from the macro
    /// Create a new asset from const paths
    pub const fn new_from_const(
        absolute_source_path: ConstStr,
        bundled_path: ConstStr,
        options: AssetOptions,
    ) -> Self {
        Self {
            absolute_source_path,
            bundled_path,
            options,
        }
    }

    /// Get the bundled name of the asset. This identifier cannot be used to read the asset directly
    pub fn bundled_path(&self) -> &str {
        self.bundled_path.as_str()
    }

    /// Get the absolute path of the asset source. This path will not be available when the asset is bundled
    pub fn absolute_source_path(&self) -> &str {
        self.absolute_source_path.as_str()
    }
    /// Get the options for the asset
    pub const fn options(&self) -> &AssetOptions {
        &self.options
    }
}

/// A bundled asset with some options. The asset can be used in rsx! to reference the asset.
/// It should not be read directly with [`std::fs::read`] because the path needs to be resolved
/// relative to the bundle
///
/// ```rust
/// # use manganis::{asset, Asset};
/// # use dioxus::prelude::*;
/// const ASSET: Asset = asset!("/assets/image.png");
/// rsx! {
///     img { src: ASSET }
/// };
/// ```
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Asset {
    /// The bundled asset
    bundled: BundledAsset,
    /// The link section for the asset
    keep_link_section: fn() -> u8,
}

impl Asset {
    #[doc(hidden)]
    /// This should only be called from the macro
    /// Create a new asset from the bundled form of the asset and the link section
    pub const fn new(bundled: BundledAsset, keep_link_section: fn() -> u8) -> Self {
        Self {
            bundled,
            keep_link_section,
        }
    }

    /// Get the bundled asset
    pub const fn bundled(&self) -> &BundledAsset {
        &self.bundled
    }

    /// Return a canonicalized path to the asset
    ///
    /// Attempts to resolve it against an `assets` folder in the current directory.
    /// If that doesn't exist, it will resolve against the cargo manifest dir
    pub fn resolve(&self) -> PathBuf {
        // Force a volatile read of the asset link section to ensure the symbol makes it into the binary
        (self.keep_link_section)();

        #[cfg(feature = "dioxus")]
        // If the asset is relative, we resolve the asset at the current directory
        if !dioxus_core_types::is_bundled_app() {
            return PathBuf::from(self.bundled.absolute_source_path.as_str());
        }

        #[cfg(feature = "dioxus")]
        let bundle_root = {
            let base_path = dioxus_cli_config::base_path();
            let base_path = base_path
                .as_deref()
                .map(|base_path| {
                    let trimmed = base_path.trim_matches('/');
                    format!("/{trimmed}")
                })
                .unwrap_or_default();
            PathBuf::from(format!("{base_path}/assets/"))
        };
        #[cfg(not(feature = "dioxus"))]
        let bundle_root = PathBuf::from("/assets/");

        // Otherwise presumably we're bundled and we can use the bundled path
        bundle_root.join(PathBuf::from(
            self.bundled.bundled_path.as_str().trim_start_matches('/'),
        ))
    }
}

impl From<Asset> for String {
    fn from(value: Asset) -> Self {
        value.to_string()
    }
}
impl From<Asset> for Option<String> {
    fn from(value: Asset) -> Self {
        Some(value.to_string())
    }
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.resolve().display())
    }
}

#[cfg(feature = "dioxus")]
impl dioxus_core_types::DioxusFormattable for Asset {
    fn format(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(self.to_string())
    }
}
