// Re-export const_serialize types for generated code.
pub use const_serialize;
pub use const_serialize::{ConstStr, ConstVec, SerializeConst};
pub use const_serialize_07;
// Re-export copy_bytes so generated code can use it without a dx-macro-helpers dependency.
pub use dx_macro_helpers::copy_bytes;

use const_serialize::serialize_const;
use const_serialize_07::{
    serialize_const as serialize_const_07, ConstVec as ConstVec07,
    SerializeConst as SerializeConst07,
};
use manganis_core::{AssetOptions, BundledAsset};

/// Create a bundled asset from the input path, the content hash, and the asset options
pub const fn create_bundled_asset(input_path: &str, asset_config: AssetOptions) -> BundledAsset {
    BundledAsset::new(input_path, BundledAsset::PLACEHOLDER_HASH, asset_config)
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
    // Serialize using the default buffer, then expand into the fixed-size buffer.
    let serialized = serialize_const(asset, const_serialize::ConstVec::new());
    let mut data: ConstVec<u8, 4096> = ConstVec::new_with_max_size();
    data = data.extend(serialized.as_ref());
    // Pad to full buffer size (4096) to match linker section size.
    while data.len() < 4096 {
        data = data.push(0);
    }
    data
}

/// Serialize an asset to a const buffer in the legacy 0.7 format
pub const fn serialize_asset_07(asset: &BundledAsset) -> ConstVec07<u8> {
    let data = ConstVec07::new();
    let mut data = serialize_const_07(asset, data);
    // Reserve the maximum size of the asset
    while data.len() < <BundledAsset as SerializeConst07>::MEMORY_LAYOUT.size() {
        data = data.push(0);
    }
    data
}

/// Deserialize a const buffer into a BundledAsset
pub const fn deserialize_asset(bytes: &[u8]) -> BundledAsset {
    match const_serialize::deserialize_const!(BundledAsset, bytes) {
        Some((_, asset)) => asset,
        None => panic!("Failed to deserialize asset. This may be caused by a mismatch between your dioxus and dioxus-cli versions"),
    }
}
