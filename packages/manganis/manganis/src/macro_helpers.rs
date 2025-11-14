// Re-export const_serialize types for convenience
pub use const_serialize::{self, ConstStr, ConstVec, SerializeConst};
// Re-export copy_bytes so generated code can use it without dx-macro-helpers dependency
pub use dx_macro_helpers::copy_bytes;
use manganis_core::{AssetOptions, BundledAsset};

const PLACEHOLDER_HASH: &str = "This should be replaced by dx as part of the build process. If you see this error, make sure you are using a matching version of dx and dioxus and you are not stripping symbols from your binary.";

/// Create a bundled asset from the input path, the content hash, and the asset options
pub const fn create_bundled_asset(input_path: &str, asset_config: AssetOptions) -> BundledAsset {
    BundledAsset::new(input_path, PLACEHOLDER_HASH, asset_config)
}

/// Create a bundled asset from the input path, the content hash, and the asset options with a relative asset deprecation warning
///
/// This method is deprecated and will be removed in a future release.
#[deprecated(
    note = "Relative asset!() paths are not supported. Use a path like `/assets/myfile.png` instead of `./assets/myfile.png`"
)]
pub const fn create_bundled_asset_relative(
    input_path: &str,
    asset_config: AssetOptions,
) -> BundledAsset {
    create_bundled_asset(input_path, asset_config)
}

/// Serialize an asset to a const buffer
///
/// Serializes the asset directly (not wrapped in SymbolData) for simplicity.
/// Uses a 4096-byte buffer to accommodate assets with large data.
/// The buffer is padded to the full buffer size (4096) to match the
/// linker section size. const-serialize deserialization will ignore
/// the padding (zeros) at the end.
pub const fn serialize_asset(asset: &BundledAsset) -> ConstVec<u8, 4096> {
    // Serialize using the default buffer, then expand into the fixed-size buffer
    let serialized = const_serialize::serialize_const(asset, ConstVec::new());
    let mut data: ConstVec<u8, 4096> = ConstVec::new_with_max_size();
    data = data.extend(serialized.as_ref());
    // Pad to full buffer size (4096) to match linker section size
    while data.len() < 4096 {
        data = data.push(0);
    }
    data
}

/// Deserialize a const buffer into a BundledAsset
pub const fn deserialize_asset(bytes: &[u8]) -> BundledAsset {
    let bytes = ConstVec::new().extend(bytes);
    match const_serialize::deserialize_const!(BundledAsset, bytes.as_ref()) {
        Some((_, asset)) => asset,
        None => panic!("Failed to deserialize asset. This may be caused by a mismatch between your dioxus and dioxus-cli versions"),
    }
}
