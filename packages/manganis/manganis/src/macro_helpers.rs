pub use const_serialize;
use const_serialize::{serialize_const, ConstVec, SerializeConst};
use manganis_core::{AssetOptions, BundledAsset};

const PLACEHOLDER_HASH: &str = "this is a placeholder path which will be replaced by the linker";

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
pub const fn serialize_asset(asset: &BundledAsset) -> ConstVec<u8> {
    let data = ConstVec::new();
    let mut data = serialize_const(asset, data);
    // Reserve the maximum size of the asset
    while data.len() < BundledAsset::MEMORY_LAYOUT.size() {
        data = data.push(0);
    }
    data
}

/// Deserialize a const buffer into a BundledAsset
pub const fn deserialize_asset(bytes: &[u8]) -> BundledAsset {
    let bytes = ConstVec::new().extend(bytes);
    match const_serialize::deserialize_const!(BundledAsset, bytes.read()) {
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
