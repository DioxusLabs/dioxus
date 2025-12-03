pub use const_serialize;
use const_serialize::{serialize_const, ConstVec, SerializeConst};
pub use const_serialize_07;
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
pub const fn serialize_asset(asset: &BundledAsset) -> ConstVec<u8> {
    let data = ConstVec::new();
    let mut data = serialize_const(asset, data);
    // Reserve the maximum size of the asset
    while data.len() < <BundledAsset as SerializeConst>::MEMORY_LAYOUT.size() {
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

/// Copy a slice into a constant sized buffer at compile time
pub const fn copy_bytes<const N: usize>(bytes: &[u8]) -> [u8; N] {
    let mut out = [0; N];
    let mut i = 0;
    while i < N {
        out[i] = bytes[i];
        i += 1;
    }
    out
}
