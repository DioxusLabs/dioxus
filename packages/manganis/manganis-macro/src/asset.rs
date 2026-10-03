use crate::{AssetParseError, PathResolver, linker::generate_link_section};
use macro_string::MacroString;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, quote};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};
use syn::{
    Token,
    parse::{Parse, ParseStream},
    spanned::Spanned as _,
};

pub struct AssetParser {
    /// The token(s) of the source string, for error reporting
    pub(crate) path_expr: proc_macro2::TokenStream,

    /// The asset itself
    pub(crate) asset: Result<PathBuf, AssetParseError>,

    /// The source of the trailing options
    pub(crate) options: TokenStream2,
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
    //      AssetOptions::image()
    //        .format(ImageFormat::Jpg)
    //        .size(512, 512)
    // )
    // ```
    //
    // But we need to decide the hint first before parsing the options
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // And then parse the options
        let (MacroString(src), path_expr) = input.call(crate::parse_with_tokens)?;
        let asset = PathResolver::new(&src, &path_expr.span()).resolve();
        let _comma = input.parse::<Token![,]>();
        let options = input.parse()?;

        Ok(Self {
            path_expr,
            asset,
            options,
        })
    }
}

impl ToTokens for AssetParser {
    // The manganis macro outputs info to two different places:
    // 1) The crate the macro was invoked in
    //   - It needs the hashed contents of the file, the file path, and the file options
    //   - Most of this is just forwarding the input, the only thing that the macro needs to do is hash the file contents
    // 2) A bundler that supports manganis (currently just dioxus-cli)
    //   - The macro needs to output the absolute path to the asset for the bundler to find later
    //   - It also needs to serialize the bundled asset along with the asset options for the bundler to use later
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self.asset.as_ref() {
            Ok(asset) => tokens.extend(self.expand_asset_tokens(asset)),
            Err(err) => tokens.extend(self.error_tokens(err)),
        }
    }
}

impl AssetParser {
    pub(crate) fn expand_asset_tokens(&self, asset: &Path) -> proc_macro2::TokenStream {
        let asset_string = asset.to_string_lossy();
        if let Some(error) = asset_path_length_error(&asset_string) {
            return syn::Error::new(self.path_expr.span(), error).into_compile_error();
        }
        let mut asset_str = proc_macro2::Literal::string(&asset_string);
        asset_str.set_span(self.path_expr.span());

        let mut hash = DefaultHasher::new();
        format!("{:?}", self.options.span()).hash(&mut hash);
        format!("{:?}", self.options.to_string()).hash(&mut hash);
        asset_string.hash(&mut hash);
        let asset_hash = format!("{:016x}", hash.finish());

        // Generate the link section for the asset. The link section includes the source path and the
        // output path of the asset. We force the asset to be included in the binary even if it is unused
        // if the asset is unhashed
        let link_section = generate_link_section(quote!(__ASSET), &asset_hash);

        // generate the asset::new method to deprecate the `./assets/blah.css` syntax
        let constructor = if asset.is_relative() {
            quote! { create_bundled_asset_relative }
        } else {
            quote! { create_bundled_asset }
        };

        let options = if self.options.is_empty() {
            quote! { manganis::AssetOptions::builder() }
        } else {
            self.options.clone()
        };

        quote! {
            {
                // The source is used by the CLI to copy the asset
                const __ASSET_SOURCE_PATH: &'static str = #asset_str;
                // The options give the CLI info about how to process the asset
                // Note: into_asset_options is not a trait, so we cannot accept the options directly
                // in the constructor. Stable rust doesn't have support for constant functions in traits
                const __ASSET_OPTIONS: manganis::AssetOptions = #options.into_asset_options();
                // The input token hash is used to uniquely identify the link section for this asset
                const __ASSET_HASH: &'static str = #asset_hash;
                // Create the asset that the crate will use. This is used both in the return value and
                // added to the linker for the bundler to copy later
                const __ASSET: manganis::BundledAsset = manganis::macro_helpers::#constructor(__ASSET_SOURCE_PATH, __ASSET_OPTIONS);

                #link_section

                manganis::Asset::new(
                    || unsafe { std::ptr::read_volatile(&__LINK_SECTION) },
                )
            }
        }
    }

    pub(crate) fn expand_option_tokens(&self) -> proc_macro2::TokenStream {
        match self.asset.as_ref() {
            Ok(asset) => {
                let asset_tokens = self.expand_asset_tokens(asset);
                quote! { ::core::option::Option::Some(#asset_tokens) }
            }
            Err(AssetParseError::DoesNotExist { .. }) => {
                quote! { ::core::option::Option::<manganis::Asset>::None }
            }
            Err(err) => self.error_tokens(err),
        }
    }

    fn error_tokens(&self, err: &AssetParseError) -> proc_macro2::TokenStream {
        let err = err.to_string();
        quote! { compile_error!(#err) }
    }
}

fn asset_path_length_error(asset_path: &str) -> Option<String> {
    let actual = asset_path.len();
    let maximum = manganis_core::BundledAsset::MAX_PATH_LEN;
    if actual <= maximum {
        return None;
    }

    let excess = actual - maximum;
    let unit = if excess == 1 { "byte" } else { "bytes" };
    Some(format!(
        "Asset path is {actual} bytes long, exceeding the {maximum}-byte limit by {excess} {unit}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_path_at_maximum_length_is_valid() {
        let path = "a".repeat(manganis_core::BundledAsset::MAX_PATH_LEN);
        assert_eq!(asset_path_length_error(&path), None);
    }

    #[test]
    fn asset_path_over_maximum_length_has_actionable_error() {
        let maximum = manganis_core::BundledAsset::MAX_PATH_LEN;
        let actual = maximum + 1;
        let path = "a".repeat(actual);
        let expected = format!(
            "Asset path is {actual} bytes long, exceeding the {maximum}-byte limit by 1 byte"
        );

        assert_eq!(asset_path_length_error(&path), Some(expected.clone()));

        let parser = AssetParser {
            path_expr: quote::quote!("/assets/asset.txt"),
            asset: Ok(path.clone().into()),
            options: TokenStream2::new(),
        };
        let expanded = parser.expand_asset_tokens(Path::new(&path)).to_string();
        assert!(expanded.contains("compile_error"));
        assert!(expanded.contains(&expected));
    }
}
