#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use std::{
    hash::Hasher,
    io::Read,
    path::{Path, PathBuf},
};

use css_module::CssModuleParser;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
};

pub(crate) mod asset;
pub(crate) mod css_module;
pub(crate) mod linker;

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

/// Generate type-safe and globally-unique CSS identifiers from a CSS module.
///
/// CSS modules allow you to have unique, scoped and type-safe CSS identifiers. A CSS module is a CSS file with the `.module.css` file extension.
/// The `css_module!()` macro allows you to utilize CSS modules in your Rust projects.
///
/// # Syntax
///
/// The `css_module!()` macro takes a few items.
/// - A styles struct identifier. This is the `struct` you use to access your type-safe CSS identifiers in Rust.
/// - The asset string path. This is the absolute path (from the crate root) to your CSS module.
/// - An optional `CssModuleAssetOptions` struct to configure the processing of your CSS module.
///
/// ```rust
/// css_module!(StylesIdent = "/my.module.css", CssModuleAssetOptions::new());
/// ```
///
/// The styles struct can be made public by appending `pub` before the identifier.
/// Read the [Variable Visibility](#variable-visibility) section for more information.
///
/// # Generation
///
/// The `css_module!()` macro does two few things:
/// - It generates an asset using the `asset!()` macro and automatically inserts it into the app meta.
/// - It generates a struct with snake-case associated constants of your CSS idents.
///
/// ```rust
/// // This macro usage:
/// css_module!(Styles = "/mycss.module.css");
///
/// // Will generate this (simplified):
/// struct Styles {}
///
/// impl Styles {
///     // This can be accessed with `Styles::your_ident`
///     pub const your_ident: &str = "abc",
/// }
/// ```
///
/// # CSS Identifier Collection
/// The macro will collect all identifiers used in your CSS module, convert them into snake_case, and generate a struct and fields around those identifier names.
///
/// For example, `#fooBar` will become `foo_bar`.
///
/// Identifier used only inside of a media query, will not be collected (not yet supported). To get around this, you can use an empty block for the identifier:
/// ```css
/// /* Empty ident block to ensure collection */
/// #foo {}
///
/// @media ... {
///     #foo { ... }
/// }
/// ```
///
/// # Variable Visibility
/// If you want your asset or styles constant to be public, you can add the `pub` keyword in front of them.
/// Restricted visibility (`pub(super)`, `pub(crate)`, etc) is also supported.
/// ```rust
/// css_module!(pub Styles = "/mycss.module.css");
/// ```
///
/// # Asset Options
/// Similar to the  `asset!()` macro, you can pass an optional `CssModuleAssetOptions` to configure a few processing settings.
/// ```rust
/// use manganis::CssModuleAssetOptions;
///
/// css_module!(Styles = "/mycss.module.css",
///     CssModuleAssetOptions::new()
///         .with_minify(true)
///         .with_preload(false),
/// );
/// ```
///
/// # Examples
/// First you need a CSS module:
/// ```css
/// /* mycss.module.css */
///
/// #header {
///     padding: 50px;
/// }
///
/// .header {
///     margin: 20px;
/// }
///
/// .button {
///     background-color: #373737;        
/// }
/// ```
/// Then you can use the `css_module!()` macro in your Rust project:
/// ```rust
/// css_module!(Styles = "/mycss.module.css");
///
/// println!("{}", Styles::header);
/// println!("{}", Styles::header_class);
/// println!("{}", Styles::button);
/// ```
#[proc_macro]
pub fn css_module(input: TokenStream) -> TokenStream {
    let style = parse_macro_input!(input as CssModuleParser);

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
