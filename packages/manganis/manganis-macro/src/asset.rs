use crate::{resolve_path, AssetParseError};
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use std::path::PathBuf;
use syn::{
    parse::{Parse, ParseStream},
    LitStr, Token,
};

pub struct AssetParser {
    /// The span of the source string
    pub(crate) path_span: proc_macro2::Span,

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
    //      ImageAssetOptions::new()
    //        .format(ImageFormat::Jpg)
    //        .size(512, 512)
    // )
    // ```
    //
    // But we need to decide the hint first before parsing the options
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // And then parse the options
        let src = input.parse::<LitStr>()?;
        let path_span = src.span();
        let asset = resolve_path(&src.value());
        let _comma = input.parse::<Token![,]>();
        let options = input.parse()?;

        Ok(Self {
            path_span,
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
        let asset = match self.asset.as_ref() {
            Ok(asset) => asset,
            Err(err) => {
                let err = err.to_string();
                tokens.append_all(quote! { compile_error!(#err) });
                return;
            }
        };
        let asset_str = asset.display().to_string();
        let mut asset_str = proc_macro2::Literal::string(&asset_str);
        asset_str.set_span(self.path_span);

        let hash = match crate::hash_file_contents(asset) {
            Ok(hash) => hash,
            Err(err) => {
                let err = err.to_string();
                tokens.append_all(quote! { compile_error!(#err) });
                return;
            }
        };

        // Generate the link section for the asset
        // The link section includes the source path and the output path of the asset
        let link_section = crate::generate_link_section(quote!(__ASSET));

        // generate the asset::new method to deprecate the `./assets/blah.css` syntax
        let constructor = if asset.is_relative() {
            quote::quote! { new_relative }
        } else {
            quote::quote! { new }
        };

        let options = if self.options.is_empty() {
            quote! { manganis::AssetOptions::Unknown }
        } else {
            self.options.clone()
        };

        tokens.extend(quote! {
            {
                // We keep a hash of the contents of the asset for cache busting
                const __ASSET_HASH: u64 = #hash;
                // The source is used by the CLI to copy the asset
                const __ASSET_SOURCE_PATH: &'static str = #asset_str;
                // The options give the CLI info about how to process the asset
                // Note: into_asset_options is not a trait, so we cannot accept the options directly
                // in the constructor. Stable rust doesn't have support for constant functions in traits
                const __ASSET_OPTIONS: manganis::AssetOptions = #options.into_asset_options();
                // We calculate the bundled path from the hash and any transformations done by the options
                // This is the final path that the asset will be written to
                const __ASSET_BUNDLED_PATH: manganis::macro_helpers::const_serialize::ConstStr = manganis::macro_helpers::generate_unique_path(__ASSET_SOURCE_PATH, __ASSET_HASH, &__ASSET_OPTIONS);
                // Get the reference to the string that was generated. We cannot return &'static str from
                // generate_unique_path because it would return a reference to data generated in the function
                const __ASSET_BUNDLED_PATH_STR: &'static str = __ASSET_BUNDLED_PATH.as_str();
                // Create the asset that the crate will use. This is used both in the return value and
                // added to the linker for the bundler to copy later
                const __ASSET: manganis::BundledAsset = manganis::BundledAsset::#constructor(__ASSET_SOURCE_PATH, __ASSET_BUNDLED_PATH_STR, __ASSET_OPTIONS);

                #link_section

                manganis::Asset::new(__ASSET, __keep_link_section)
            }
        })
    }
}
