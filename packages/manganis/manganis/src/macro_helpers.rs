// Re-export const_serialize types for generated code.
pub use const_serialize;
pub use const_serialize::{ConstStr, ConstVec, SerializeConst};
pub use const_serialize_07;

// Re-export dx-macro-helpers so generated code can use it without a direct dependency.
pub use dx_macro_helpers;
pub use dx_macro_helpers::copy_bytes;

use const_serialize_07::ConstVec as ConstVec07;
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
/// Uses a 4096-byte buffer and pads to the full size to match linker section size.
pub const fn serialize_asset(asset: &BundledAsset) -> ConstVec<u8, 4096> {
    dx_macro_helpers::serialize_to_const_with_max_padded::<4096>(asset)
}

/// Serialize an asset to a const buffer in the legacy 0.7 format
pub const fn serialize_asset_07(asset: &BundledAsset) -> ConstVec07<u8> {
    dx_macro_helpers::serialize_to_const_with_layout_padded_07(asset)
}

/// Deserialize a const buffer into a BundledAsset
pub const fn deserialize_asset(bytes: &[u8]) -> BundledAsset {
    match const_serialize::deserialize_const!(BundledAsset, bytes) {
        Some((_, asset)) => asset,
        None => panic!("Failed to deserialize asset. This may be caused by a mismatch between your dioxus and dioxus-cli versions"),
    }
}
