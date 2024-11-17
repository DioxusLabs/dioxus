use manganis_core::{AssetParseError, ResourceAsset};
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    LitStr,
};

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
        let asset = ResourceAsset::parse_any(&src.value());
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
        let method = if !asset.input.starts_with("/") {
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

                        metadata: __volatile_reader as fn() -> u8,
                    }
                }
            ) #option_source
        })
    }
}
