// Re-export const_serialize types for generated code.
pub use const_serialize;
pub use const_serialize::{ConstStr, ConstVec, SerializeConst};
pub use const_serialize_07;

use const_serialize_07::ConstVec as ConstVec07;
use manganis_core::{
    AndroidArtifactMetadata, AssetOptions, BundledAsset, SwiftPackageMetadata, SymbolData,
};

/// Copy a slice into a constant sized buffer at compile time
///
/// This is a generic utility that works with any byte slice and can be used
/// in const contexts to create fixed-size arrays from dynamic slices.
pub const fn copy_bytes<const N: usize>(bytes: &[u8]) -> [u8; N] {
    let mut out = [0; N];
    let mut i = 0;
    while i < N {
        out[i] = bytes[i];
        i += 1;
    }
    out
}

/// Serialize a SymbolData value into a const buffer
///
/// This is used by the widget!() macro and other symbol-based macros to embed
/// metadata into the binary using the 4096-byte buffer format.
pub const fn serialize_symbol_data(symbol_data: &SymbolData) -> ConstVec<u8, 4096> {
    dx_macro_helpers::serialize_to_const_with_max_padded::<4096>(symbol_data)
}

/// Serialize Android artifact metadata (wrapped in `SymbolData::AndroidArtifact`).
pub const fn serialize_android_artifact(meta: &AndroidArtifactMetadata) -> ConstVec<u8, 4096> {
    serialize_symbol_data(&SymbolData::AndroidArtifact(*meta))
}

/// Serialize Swift package metadata (wrapped in `SymbolData::SwiftPackage`).
pub const fn serialize_swift_package(meta: &SwiftPackageMetadata) -> ConstVec<u8, 4096> {
    serialize_symbol_data(&SymbolData::SwiftPackage(*meta))
}

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

pub mod dx_macro_helpers {
    use const_serialize::{ConstVec, SerializeConst};

    /// Serialize a value to a const buffer, padding to the specified size
    ///
    /// This is a generic helper that works with any type implementing `SerializeConst`.
    /// It serializes the value and then pads the buffer to the specified memory layout size.
    pub const fn serialize_to_const<T: SerializeConst>(
        value: &T,
        memory_layout_size: usize,
    ) -> ConstVec<u8> {
        let data = ConstVec::new();
        let mut data = const_serialize::serialize_const(value, data);
        // Reserve the maximum size of the type
        while data.len() < memory_layout_size {
            data = data.push(0);
        }
        data
    }

    /// Serialize a value to a const buffer with a fixed maximum size, padding to the specified size
    ///
    /// This variant uses a `ConstVec` with a fixed maximum size (e.g., `ConstVec<u8, 4096>`)
    /// and then pads to the specified memory layout size.
    ///
    /// This function serializes directly into the larger buffer to avoid overflow issues
    /// when the serialized data exceeds the default 1024-byte buffer size.
    pub const fn serialize_to_const_with_max<const MAX_SIZE: usize>(
        value: &impl SerializeConst,
        memory_layout_size: usize,
    ) -> ConstVec<u8, MAX_SIZE> {
        // Serialize using the default buffer, then copy into the larger buffer
        let serialized = const_serialize::serialize_const(value, ConstVec::new());
        let mut data: ConstVec<u8, MAX_SIZE> = ConstVec::new_with_max_size();
        data = data.extend(serialized.as_ref());
        // Reserve the maximum size of the type (pad to MEMORY_LAYOUT size)
        while data.len() < memory_layout_size {
            data = data.push(0);
        }
        data
    }

    /// Serialize a value to a const buffer and pad to the full buffer size
    ///
    /// This is useful for linker section generation that expects a fixed-size buffer.
    pub const fn serialize_to_const_with_max_padded<const MAX_SIZE: usize>(
        value: &impl SerializeConst,
    ) -> ConstVec<u8, MAX_SIZE> {
        let serialized = const_serialize::serialize_const(value, ConstVec::new());
        let mut data: ConstVec<u8, MAX_SIZE> = ConstVec::new_with_max_size();
        data = data.extend(serialized.as_ref());
        while data.len() < MAX_SIZE {
            data = data.push(0);
        }
        data
    }

    /// Serialize a value using the legacy 0.7 const-serialize format and pad to layout size
    ///
    /// Note: The legacy ConstVec has a 1024-byte limit. If MEMORY_LAYOUT.size() exceeds this,
    /// we pad only up to the buffer limit to avoid overflow.
    pub const fn serialize_to_const_with_layout_padded_07<T: const_serialize_07::SerializeConst>(
        value: &T,
    ) -> const_serialize_07::ConstVec<u8> {
        let data = const_serialize_07::ConstVec::new();
        let mut data = const_serialize_07::serialize_const(value, data);
        // Pad to MEMORY_LAYOUT size, but cap at 1024 bytes (the legacy buffer limit)
        let target_size = if T::MEMORY_LAYOUT.size() > 1024 {
            1024
        } else {
            T::MEMORY_LAYOUT.size()
        };
        while data.len() < target_size {
            data = data.push(0);
        }
        data
    }
}
