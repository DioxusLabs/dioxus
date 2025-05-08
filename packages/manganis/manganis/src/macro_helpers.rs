pub use const_serialize;
use const_serialize::{serialize_const, ConstStr, ConstVec};
use manganis_core::{AssetOptions, BundledAsset};

const PLACEHOLDER_HASH: ConstStr =
    ConstStr::new("this is a placeholder path which will be replaced by the linker");

/// Create a bundled asset from the input path, the content hash, and the asset options
pub const fn create_bundled_asset(
    input_path: &str,
    asset_config: AssetOptions,
    link_section: &str,
) -> BundledAsset {
    BundledAsset::new_from_const(
        ConstStr::new(input_path),
        PLACEHOLDER_HASH,
        asset_config,
        ConstStr::new(link_section),
    )
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
    link_section: &str,
) -> BundledAsset {
    create_bundled_asset(input_path, asset_config, link_section)
}

/// Serialize an asset to a const buffer
pub const fn serialize_asset(asset: &BundledAsset) -> ConstVec<u8> {
    let write = ConstVec::new();
    serialize_const(asset, write)
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
