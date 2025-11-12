use const_serialize::SerializeConst;
use manganis_core::BundledAsset;

use crate::Permission;

/// Unified symbol data that can represent both assets and permissions
///
/// This enum is used to serialize different types of metadata into the binary
/// using the same `__ASSETS__` symbol prefix. The CBOR format allows for
/// self-describing data, making it easy to add new variants in the future.
///
/// Variant order does NOT matter for CBOR enum serialization - variants are
/// matched by name (string), not by position or tag value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, SerializeConst)]
#[repr(C, u8)]
pub enum SymbolData {
    /// An asset that should be bundled with the application
    Asset(BundledAsset),
    /// A permission declaration for the application
    Permission(Permission),
}

