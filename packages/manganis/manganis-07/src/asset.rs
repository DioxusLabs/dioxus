use crate::AssetOptions;
use const_serialize_07::{ConstStr, SerializeConst};
use std::{fmt::Debug, hash::Hash};

/// An asset that should be copied by the bundler with some options. This type will be
/// serialized into the binary.
/// CLIs that support manganis, should pull out the assets from the link section, optimize,
/// and write them to the filesystem at [`BundledAsset::bundled_path`] for the application
/// to use.
#[derive(Debug, Eq, Clone, Copy, SerializeConst, serde::Serialize, serde::Deserialize)]
pub struct BundledAsset {
    /// The absolute path of the asset
    absolute_source_path: ConstStr,
    /// The bundled path of the asset
    bundled_path: ConstStr,
    /// The options for the asset
    options: AssetOptions,
}

impl PartialEq for BundledAsset {
    fn eq(&self, other: &Self) -> bool {
        self.absolute_source_path == other.absolute_source_path
            && self.bundled_path == other.bundled_path
            && self.options == other.options
    }
}

impl PartialOrd for BundledAsset {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self
            .absolute_source_path
            .partial_cmp(&other.absolute_source_path)
        {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.bundled_path.partial_cmp(&other.bundled_path) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.options.partial_cmp(&other.options)
    }
}

impl Hash for BundledAsset {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.absolute_source_path.hash(state);
        self.bundled_path.hash(state);
        self.options.hash(state);
    }
}

impl BundledAsset {
    #[doc(hidden)]
    /// This should only be called from the macro
    /// Create a new asset
    pub const fn new(
        absolute_source_path: &str,
        bundled_path: &str,
        options: AssetOptions,
    ) -> Self {
        Self {
            absolute_source_path: ConstStr::new(absolute_source_path),
            bundled_path: ConstStr::new(bundled_path),
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
