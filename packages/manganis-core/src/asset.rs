use crate::AssetOptions;
use const_serialize::{ConstStr, SerializeConst};
use std::path::PathBuf;

/// A bundled asset with some options. You need to
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
pub struct Asset {
    /// The absolute path of the asset
    absolute_source_path: ConstStr,
    /// The bundled path of the asset
    bundled_path: ConstStr,
    /// The options for the asset
    options: AssetOptions,
}

impl Asset {
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

    /// Get the bundled name of the asset. This identifier cannot be used to read the asset directly
    pub fn bundled_path(&self) -> &str {
        self.bundled_path.as_str()
    }

    /// Get the absolute path of the asset source. This path will not be available when the asset is bundled
    pub fn absolute_source_path(&self) -> &str {
        self.absolute_source_path.as_str()
    }

    /// Return a canonicalized path to the asset
    ///
    /// Attempts to resolve it against an `assets` folder in the current directory.
    /// If that doesn't exist, it will resolve against the cargo manifest dir
    pub fn resolve(&self) -> PathBuf {
        #[cfg(feature = "dioxus")]
        // If the asset is relative, we resolve the asset at the current directory
        if !dioxus_core_types::is_bundled_app() {
            return PathBuf::from(self.absolute_source_path.as_str());
        }

        // Otherwise presumably we're bundled and we can use the bundled path
        PathBuf::from("/assets/").join(PathBuf::from(
            self.bundled_path.as_str().trim_start_matches('/'),
        ))
    }

    /// Get the options for the asset
    pub const fn options(&self) -> &AssetOptions {
        &self.options
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
