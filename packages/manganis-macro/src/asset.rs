use core::panic;
use manganis_core::ResourceAsset;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned, ToTokens};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, path::Path, sync::atomic::AtomicBool};
use std::{path::PathBuf, sync::atomic::Ordering};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Token,
    Expr, ExprLit, Lit, LitStr, PatLit, Token,
};

use crate::asset_options::MethodCallOption;

pub struct AssetParser {
    /// The source of the trailing builder pattern
    option_source: TokenStream2,

    /// The asset itself
    asset: ResourceAsset,
}

impl Parse for AssetParser {
    // we can take
    //
    // This gives you the Asset type - it's generic and basically unrefined
    // ```
    // asset!("myfile.png")
    // ```
    //
    // To narrow the type, use a method call to get the refined type
    // ```
    // asset!(
    //     "myfile.png"
    //      .image()
    //      .format(ImageType::Jpg)
    //      .size(512, 512)
    // )
    // ```
    //
    // But we need to decide the hint first before parsing the options
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Get the source of the macro, excluding the first token
        let option_source = {
            let fork = input.fork();
            fork.parse::<LitStr>()?;
            fork.parse::<TokenStream2>()?
        };

        // And then parse the options
        let src = input.parse::<LitStr>()?;
        let src = src.value();
        let resource = ResourceAsset::parse_any(&src).unwrap();

        fn parse_call(input: ParseStream) -> syn::Result<MethodCallOption> {
            let method = input.parse::<syn::Ident>()?;
            let content;
            parenthesized!(content in input);

            let args = Punctuated::<Lit, Token![,]>::parse_separated_nonempty(&content)?;

            Ok(MethodCallOption { method, args })
        }

        let mut options = vec![];

        while !input.is_empty() {
            let option = parse_call(input);
            if let Ok(option) = option {
                options.push(option);
            } else {
                // todo: make sure we toss a warning in the output
                let _remaining: TokenStream2 = input.parse()?;
            }
        }

        Ok(Self {
            option_source,
            asset: resource,
        })
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
        // 1. the link section itself
        let link_section = crate::generate_link_section(&self.asset);

        // 2. original
        let input = self.asset.input.display().to_string();

        // 3. resolved on the user's system
        let local = self.asset.absolute.display().to_string();

        // 4. bundled
        let bundled = self.asset.bundled.to_string();

        // 5. source tokens
        let option_source = &self.option_source;

        tokens.extend(quote! {
            Asset::new(
                {
                    #link_section
                    manganis::Asset {
                        // "/assets/blah.css"
                        input: #input,

                        // "/users/dioxus/dev/app/assets/blah.css"
                        local: #local,

                        bundled: #bundled,
                    }
                }
            ) #option_source
        })
    }
}
