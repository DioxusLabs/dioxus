#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use std::{
    hash::Hasher,
    io::Read,
    path::{Path, PathBuf},
};

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use styles::StyleParser;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
};

pub(crate) mod asset;
pub(crate) mod linker;
pub(crate) mod styles;

use linker::generate_link_section;

/// The asset macro collects assets that will be included in the final binary
///
/// # Files
///
/// The file builder collects an arbitrary file. Relative paths are resolved relative to the package root
/// ```rust
/// # use manganis::{asset, Asset};
/// const _: Asset = asset!("/assets/asset.txt");
/// ```
/// Macros like `concat!` and `env!` are supported in the asset path.
/// ```rust
/// # use manganis::{asset, Asset};
/// const _: Asset = asset!(concat!("/assets/", env!("CARGO_CRATE_NAME"), ".dat"));
/// ```
///
/// # Images
///
/// You can collect images which will be automatically optimized with the image builder:
/// ```rust
/// # use manganis::{asset, Asset};
/// const _: Asset = asset!("/assets/image.png");
/// ```
/// Resize the image at compile time to make the assets file size smaller:
/// ```rust
/// # use manganis::{asset, Asset, ImageAssetOptions, ImageSize};
/// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_size(ImageSize::Manual { width: 52, height: 52 }));
/// ```
/// Or convert the image at compile time to a web friendly format:
/// ```rust
/// # use manganis::{asset, Asset, ImageAssetOptions, ImageSize, ImageFormat};
/// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_format(ImageFormat::Avif));
/// ```
/// You can mark images as preloaded to make them load faster in your app
/// ```rust
/// # use manganis::{asset, Asset, ImageAssetOptions};
/// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_preload(true));
/// ```
#[proc_macro]
pub fn asset(input: TokenStream) -> TokenStream {
    let asset = parse_macro_input!(input as asset::AssetParser);

    quote! { #asset }.into_token_stream().into()
}

/// styles
#[proc_macro]
pub fn styles(input: TokenStream) -> TokenStream {
    let style = parse_macro_input!(input as StyleParser);

    quote! { #style }.into_token_stream().into()
}

fn resolve_path(raw: &str) -> Result<PathBuf, AssetParseError> {
    // Get the location of the root of the crate which is where all assets are relative to
    //
    // IE
    // /users/dioxus/dev/app/
    // is the root of
    // /users/dioxus/dev/app/assets/blah.css
    let manifest_dir = dunce::canonicalize(
        std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap(),
    )
    .unwrap();

    // 1. the input file should be a pathbuf
    let input = PathBuf::from(raw);

    // 2. absolute path to the asset
    let Ok(path) = std::path::absolute(manifest_dir.join(raw.trim_start_matches('/'))) else {
        return Err(AssetParseError::InvalidPath {
            path: input.clone(),
        });
    };

    // 3. Ensure the path exists
    let Ok(path) = dunce::canonicalize(path) else {
        return Err(AssetParseError::AssetDoesntExist {
            path: input.clone(),
        });
    };

    // 4. Ensure the path doesn't escape the crate dir
    //
    // - Note: since we called canonicalize on both paths, we can safely compare the parent dirs.
    //   On windows, we can only compare the prefix if both paths are canonicalized (not just absolute)
    //   https://github.com/rust-lang/rust/issues/42869
    if path == manifest_dir || !path.starts_with(manifest_dir) {
        return Err(AssetParseError::InvalidPath { path });
    }

    Ok(path)
}

fn hash_file_contents(file_path: &Path) -> Result<u64, AssetParseError> {
    // Create a hasher
    let mut hash = std::collections::hash_map::DefaultHasher::new();

    // If this is a folder, hash the folder contents
    if file_path.is_dir() {
        let files = std::fs::read_dir(file_path).map_err(|err| AssetParseError::IoError {
            err,
            path: file_path.to_path_buf(),
        })?;
        for file in files.flatten() {
            let path = file.path();
            hash_file_contents(&path)?;
        }
        return Ok(hash.finish());
    }

    // Otherwise, open the file to get its contents
    let mut file = std::fs::File::open(file_path).map_err(|err| AssetParseError::IoError {
        err,
        path: file_path.to_path_buf(),
    })?;

    // We add a hash to the end of the file so it is invalidated when the bundled version of the file changes
    // The hash includes the file contents, the options, and the version of manganis. From the macro, we just
    // know the file contents, so we only include that hash
    let mut buffer = [0; 8192];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(AssetParseError::FailedToReadAsset)?;
        if read == 0 {
            break;
        }
        hash.write(&buffer[..read]);
    }

    Ok(hash.finish())
}

/// Parse `T`, while also collecting the tokens it was parsed from.
fn parse_with_tokens<T: Parse>(input: ParseStream) -> syn::Result<(T, proc_macro2::TokenStream)> {
    let begin = input.cursor();
    let t: T = input.parse()?;
    let end = input.cursor();

    let mut cursor = begin;
    let mut tokens = proc_macro2::TokenStream::new();
    while cursor != end {
        let (tt, next) = cursor.token_tree().unwrap();
        tokens.extend(std::iter::once(tt));
        cursor = next;
    }

    Ok((t, tokens))
}

#[derive(Debug)]
enum AssetParseError {
    AssetDoesntExist { path: PathBuf },
    IoError { err: std::io::Error, path: PathBuf },
    InvalidPath { path: PathBuf },
    FailedToReadAsset(std::io::Error),
}

impl std::fmt::Display for AssetParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetParseError::AssetDoesntExist { path } => {
                write!(f, "Asset at {} doesn't exist", path.display())
            }
            AssetParseError::IoError { path, err } => {
                write!(f, "Failed to read file: {}; {}", path.display(), err)
            }
            AssetParseError::InvalidPath { path } => {
                write!(
                    f,
                    "Asset path {} is invalid. Make sure the asset exists within this crate.",
                    path.display()
                )
            }
            AssetParseError::FailedToReadAsset(err) => write!(f, "Failed to read asset: {}", err),
        }
    }
}
