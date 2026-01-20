use crate::AssetOptions;
use const_serialize_07::{ConstStr, SerializeConst};
use std::{fmt::Debug, hash::Hash};

#[derive(Debug, Eq, Clone, Copy, SerializeConst, serde::Serialize, serde::Deserialize)]
pub struct BundledAsset {
    absolute_source_path: ConstStr,
    bundled_path: ConstStr,
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

    pub fn bundled_path(&self) -> &str {
        self.bundled_path.as_str()
    }

    pub fn absolute_source_path(&self) -> &str {
        self.absolute_source_path.as_str()
    }

    pub const fn options(&self) -> &AssetOptions {
        &self.options
    }
}
