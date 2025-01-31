pub use const_serialize;
use const_serialize::{serialize_const, ConstStr, ConstVec};
use manganis_core::{AssetOptions, BundledAsset};

use crate::hash::ConstHasher;

/// Create a bundled asset from the input path, the content hash, and the asset options
pub const fn create_bundled_asset(
    input_path: &str,
    content_hash: &[u8],
    asset_config: AssetOptions,
) -> BundledAsset {
    let hashed_path = generate_unique_path_with_byte_hash(input_path, content_hash, &asset_config);
    BundledAsset::new_from_const(ConstStr::new(input_path), hashed_path, asset_config)
}

/// Create a bundled asset from the input path, the content hash, and the asset options with a relative asset deprecation warning
///
/// This method is deprecated and will be removed in a future release.
#[deprecated(
    note = "Relative asset!() paths are not supported. Use a path like `/assets/myfile.png` instead of `./assets/myfile.png`"
)]
pub const fn create_bundled_asset_relative(
    input_path: &str,
    content_hash: &[u8],
    asset_config: AssetOptions,
) -> BundledAsset {
    create_bundled_asset(input_path, content_hash, asset_config)
}

/// Format the input path with a hash to create an unique output path for the macro in the form `{input_path}-{hash}.{extension}`
pub const fn generate_unique_path(
    input_path: &str,
    content_hash: u64,
    asset_config: &AssetOptions,
) -> ConstStr {
    let byte_hash = content_hash.to_le_bytes();
    generate_unique_path_with_byte_hash(input_path, &byte_hash, asset_config)
}

/// Format the input path with a hash to create an unique output path for the macro in the form `{input_path}-{hash}.{extension}`
const fn generate_unique_path_with_byte_hash(
    input_path: &str,
    content_hash: &[u8],
    asset_config: &AssetOptions,
) -> ConstStr {
    // Format the unique path with the format `{input_path}-{hash}.{extension}`
    // Start with the input path
    let mut input_path = ConstStr::new(input_path);
    // Then strip the prefix from the input path. The path comes from the build platform, but
    // in wasm, we don't know what the path separator is from the build platform. We need to
    // split by both unix and windows paths and take the smallest one
    let mut extension = None;
    match (input_path.rsplit_once('/'), input_path.rsplit_once('\\')) {
        (Some((_, unix_new_input_path)), Some((_, windows_new_input_path))) => {
            input_path = if unix_new_input_path.len() < windows_new_input_path.len() {
                unix_new_input_path
            } else {
                windows_new_input_path
            };
        }
        (Some((_, unix_new_input_path)), _) => {
            input_path = unix_new_input_path;
        }
        (_, Some((_, windows_new_input_path))) => {
            input_path = windows_new_input_path;
        }
        _ => {}
    }
    if let Some((new_input_path, new_extension)) = input_path.rsplit_once('.') {
        extension = Some(new_extension);
        input_path = new_input_path;
    }
    // Then add a dash
    let mut macro_output_path = input_path.push_str("-");

    // Hash the contents along with the asset config to create a unique hash for the asset
    // When this hash changes, the client needs to re-fetch the asset
    let mut hasher = ConstHasher::new();
    hasher = hasher.write(content_hash);
    hasher = hasher.hash_by_bytes(asset_config);
    let hash = hasher.finish();

    // Then add the hash in hex form
    let hash_bytes = hash.to_le_bytes();
    let mut i = 0;
    while i < hash_bytes.len() {
        let byte = hash_bytes[i];
        let first = byte >> 4;
        let second = byte & 0x0f;
        const fn byte_to_char(byte: u8) -> char {
            match char::from_digit(byte as u32, 16) {
                Some(c) => c,
                None => panic!("byte must be a valid digit"),
            }
        }
        macro_output_path = macro_output_path.push(byte_to_char(first));
        macro_output_path = macro_output_path.push(byte_to_char(second));
        i += 1;
    }

    // Finally add the extension
    match asset_config.extension() {
        Some(extension) => {
            macro_output_path = macro_output_path.push('.');
            macro_output_path = macro_output_path.push_str(extension)
        }
        None => {
            if let Some(extension) = extension {
                macro_output_path = macro_output_path.push('.');

                let ext_bytes = extension.as_str().as_bytes();

                // Rewrite scss as css
                if bytes_equal(ext_bytes, b"scss") || bytes_equal(ext_bytes, b"sass") {
                    macro_output_path = macro_output_path.push_str("css")
                } else {
                    macro_output_path = macro_output_path.push_str(extension.as_str())
                }
            }
        }
    }

    macro_output_path
}

const fn bytes_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut i = 0;
    while i < left.len() {
        if left[i] != right[i] {
            return false;
        }
        i += 1;
    }

    true
}

#[test]
fn test_unique_path() {
    use manganis_core::{ImageAssetOptions, ImageFormat};
    use std::path::PathBuf;
    let mut input_path = PathBuf::from("some");
    input_path.push("prefix");
    input_path.push("test.png");
    let content_hash = 123456789;
    let asset_config = AssetOptions::Image(ImageAssetOptions::new().with_format(ImageFormat::Avif));
    let output_path =
        generate_unique_path(&input_path.to_string_lossy(), content_hash, &asset_config);
    assert_eq!(output_path.as_str(), "test-603a88fe296462a3.avif");

    // Changing the path without changing the contents shouldn't change the hash
    let mut input_path = PathBuf::from("some");
    input_path.push("prefix");
    input_path.push("prefix");
    input_path.push("test.png");
    let content_hash = 123456789;
    let asset_config = AssetOptions::Image(ImageAssetOptions::new().with_format(ImageFormat::Avif));
    let output_path =
        generate_unique_path(&input_path.to_string_lossy(), content_hash, &asset_config);
    assert_eq!(output_path.as_str(), "test-603a88fe296462a3.avif");

    let mut input_path = PathBuf::from("test");
    input_path.push("ing");
    input_path.push("test");
    let content_hash = 123456789;
    let asset_config = AssetOptions::Unknown;
    let output_path =
        generate_unique_path(&input_path.to_string_lossy(), content_hash, &asset_config);
    assert_eq!(output_path.as_str(), "test-c8c4cfad21cac262");

    // Just changing the content hash should change the total hash
    let mut input_path = PathBuf::from("test");
    input_path.push("ing");
    input_path.push("test");
    let content_hash = 123456780;
    let asset_config = AssetOptions::Unknown;
    let output_path =
        generate_unique_path(&input_path.to_string_lossy(), content_hash, &asset_config);
    assert_eq!(output_path.as_str(), "test-7bced03789ff865c");
}

/// Serialize an asset to a const buffer
pub const fn serialize_asset(asset: &BundledAsset) -> ConstVec<u8> {
    let write = ConstVec::new();
    serialize_const(asset, write)
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
