use const_serialize::{ConstStr, SerializeConst};

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
    /// Create a new metadata
    pub const fn new(
        key: &'static str,
        value: &'static str,
    ) -> Self {
        Self {
            key: ConstStr::new(key),
            value: ConstStr::new(value),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Metadata {
    /// The bundled metadata
    bundled: BundledMetadata,
    /// The link section for the metadata
    keep_link_section: fn() -> u8,
}



impl Metadata {
    #[doc(hidden)]
    /// This should only be called from the macro
    /// Create a new metadata
    pub const fn new(bundled: BundledMetadata, keep_link_section: fn() -> u8) -> Self {
        Self {
            bundled,
            keep_link_section,
        }
    }

    /// Get the bundled metadata
    pub const fn bundled(&self) -> &BundledMetadata {
        &self.bundled
    }
}
