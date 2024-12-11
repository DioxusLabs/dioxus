use const_serialize::{ConstStr, SerializeConst};
use std::path::PathBuf;

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
pub struct BundledMetadata {
    pub key: ConstStr,
    pub value: ConstStr,
}

impl BundledMetadata {
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
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct MetaData {
    /// The bundled metadata
    bundled: BundledMetadata,
    /// The link section for the metadata
    keep_link_section: fn() -> u8,
}



impl Metadata {
    #[doc(hidden)]
    /// This should only be called from the macro
    /// Create a new metadata
    pub const fn new(key: &'static str, value: &'static str, keep_link_section: fn() -> u8) -> Self {
        Self {
            key: ConstStr::new(key),
            value: ConstStr::new(value),
            keep_link_section,
        }
    }

    /// Get the bundled metadata
    pub const fn bundled(&self) -> &BundledMetadata {
        &self.bundled
    }
}
