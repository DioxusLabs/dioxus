use manganis_core::ResourceAsset;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use std::{hash::Hasher, io::Read, path::PathBuf};
use syn::{
    parse::{Parse, ParseStream},
    LitStr,
};

fn resolve_path(raw: &str) -> Result<PathBuf, AssetParseError> {
    // Get the location of the root of the crate which is where all assets are relative to
    //
    // IE
    // /users/dioxus/dev/app/
    // is the root of
    // /users/dioxus/dev/app/assets/blah.css
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap();

    // 1. the input file should be a pathbuf
    let input = PathBuf::from(raw);

    // 2. absolute path to the asset
    manifest_dir
        .join(raw.trim_start_matches('/'))
        .canonicalize()
        .map_err(|err| AssetParseError::AssetDoesntExist {
            err,
            path: input.clone(),
        })
}

fn hash_file_contents(file_path: PathBuf) -> u64 {
    // Create a hasher
    let mut hash = std::collections::hash_map::DefaultHasher::new();

    // Open the file to get its options
    let mut file = std::fs::File::open(&file_path).unwrap();

    // We add a hash to the end of the file so it is invalidated when the bundled version of the file changes
    // The hash includes the file contents, the options, and the version of manganis. From the macro, we just
    // know the file contents, so we only include that hash
    let mut buffer = [0; 8192];
    loop {
        let read = file.read(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        hash.write(&buffer[..read]);
    }

    hash.finish()
}

#[derive(Debug)]
pub(crate) enum AssetParseError {
    ParseError(String),
    AssetDoesntExist {
        err: std::io::Error,
        path: std::path::PathBuf,
    },
    FailedToReadAsset(std::io::Error),
}

impl std::fmt::Display for AssetParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetParseError::ParseError(err) => write!(f, "Failed to parse asset: {}", err),
            AssetParseError::AssetDoesntExist { err, path } => {
                write!(f, "Asset at {} doesn't exist: {}", path.display(), err)
            }
            AssetParseError::FailedToReadAsset(err) => write!(f, "Failed to read asset: {}", err),
        }
    }
}

pub struct AssetParser {
    /// The asset itself
    asset: Result<ResourceAsset, AssetParseError>,

    /// The source of the trailing options
    options: TokenStream2,
}

impl Parse for AssetParser {
    // we can take
    //
    // This gives you the Asset type - it's generic and basically unrefined
    // ```
    // asset!("/assets/myfile.png")
    // ```
    //
    // To narrow the type, use a method call to get the refined type
    // ```
    // asset!(
    //     "/assets/myfile.png",
    //      asset::image()
    //        .format(ImageType::Jpg)
    //        .size(512, 512)
    // )
    // ```
    //
    // But we need to decide the hint first before parsing the options
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // And then parse the options
        let src = input.parse::<LitStr>()?;
        let asset = ResourceAsset::parse_any(&src.value())?;
        let options = input.parse()?;

        Ok(Self { asset, options })
    }
}

impl ToTokens for AssetParser {
    // Need to generate:
    //
    // - 1. absolute file path on the user's system: `/users/dioxus/dev/project/assets/blah.css`
    // - 2. original input in case that's useful: `../blah.css`
    // - 3. path relative to the CARGO_MANIFEST_DIR - and then we'll add a `/`: `/assets/blah.css
    // - 4. file from which this macro was called: `/users/dioxus/dev/project/src/lib.rs`
    // - 5: The link section containing all this data
    // - 6: the input tokens such that the builder gets validated by the const code
    // - 7: the bundled name `/blahcss123.css`
    //
    // Not that we'll use everything, but at least we have this metadata for more post-processing.
    //
    // For now, `2` and `3` will be the same since we don't support relative paths... a bit of
    // a limitation from rust itself. We technically could support them but not without some hoops
    // to jump through
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let asset = match self.asset.as_ref() {
            Ok(asset) => asset,
            Err(err) => {
                let err = err.to_string();
                tokens.append_all(quote! { compile_error!(#err) });
                return;
            }
        };

        // 1. the link section itself
        let link_section = crate::generate_link_section(&asset);

        // 2. original
        let input = asset.input.display().to_string();

        // 3. resolved on the user's system
        let local = asset.absolute.display().to_string();

        // 4. bundled
        let bundled = asset.bundled.to_string();

        // 5. source tokens
        let option_source = &self.options;

        // generate the asset::new method to deprecate the `./assets/blah.css` syntax
        let method = if asset.input.is_relative() {
            quote::quote! { new_relative }
        } else {
            quote::quote! { new }
        };

        tokens.extend(quote! {
            Asset::#method(
                {
                    #link_section
                    manganis::Asset {
                        // "/assets/blah.css"
                        input: #input,

                        // "/users/dioxus/dev/app/assets/blah.css"
                        local: #local,

                        // "/blahcss123.css"
                        bundled: #bundled,
                    }
                }
            ) #option_source
        })
    }
}
