use core::panic;
use manganis_common::{
    CssOptions, FileOptions, FontOptions, ImageOptions, JsOptions, JsonOptions, MetadataAsset,
    ResourceAsset, TailwindAsset, UnknownFileOptions, VideoOptions,
};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned, ToTokens};
use serde::Serialize;
use std::{collections::HashMap, fs::File, sync::atomic::AtomicBool};
use std::{path::PathBuf, sync::atomic::Ordering};
use syn::{
    parenthesized, parse::Parse, parse_macro_input, punctuated::Punctuated, token::Token, Expr,
    ExprLit, Lit, LitStr, PatLit, Token,
};

pub struct AssetParser {
    option_source: TokenStream2,
    resource: ResourceAsset,
    name: Option<syn::Ident>,
    parsed_options: Option<FileOptions>,
}

impl Parse for AssetParser {
    // we can take
    //
    // This gives you the Asset type - it's generic and basically unrefined
    // ```
    // asset!("myfile.png")
    // ```
    //
    // To narrow the type, use a call to get the refined type
    // ```
    // asset!(
    //     image("myfile.png")
    //      .format(ImageType::Jpg)
    //      .size(512, 512)
    // )
    // ```
    //
    // But we need to decide the hint first before parsing the options
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
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

        fn parse_call(input: syn::parse::ParseStream) -> syn::Result<MethodCallOption> {
            let ident = input.parse::<syn::Ident>()?;
            let content;
            parenthesized!(content in input);

            // Parse as puncutated literals
            let lits = Punctuated::<Lit, Token![,]>::parse_separated_nonempty(&content)?;

            Ok(MethodCallOption {
                method: ident,
                args: lits,
            })
        }

        let mut options = vec![];
        let name = None;

        while !input.is_empty() {
            let option = parse_call(input);
            if let Ok(option) = option {
                options.push(option);
            } else {
                // todo: make sure we toss a warning in the output
                let remaining: TokenStream2 = input.parse()?;
            }
        }

        let parsed_options = MethodCalls::new(options);

        Ok(Self {
            option_source,
            resource,
            name,
            parsed_options,
        })
    }
}

impl ToTokens for AssetParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let option_source = &self.option_source;
        let asset = &self.resource;
        let link_section = crate::generate_link_section(&asset);
        let input = asset.input.to_string();
        let bundled = asset.bundled.to_string();

        let local = match asset.local.as_ref() {
            Some(local) => {
                let local = local.to_string();
                quote! { #local }
            }
            None => {
                todo!("relative paths are not supported yet")
                // quote! {
                //     {
                //         // ensure it exists by throwing away the include_bytes
                //         static _BLAH: &[u8] = include_bytes!(#input);

                //         // But then pass along the path
                //         concat!(env!("CARGO_MANIFEST_DIR"), "/", file!(), "/<split>/", #input)
                //     }
                // }
            }
        };

        let manifest_dir: PathBuf = std::env::var("CARGO_MANIFEST_DIR").unwrap().into();
        let displayed_manifest_dir = manifest_dir.display().to_string();

        tokens.extend(quote! {
            Asset::new(
                {
                    #link_section
                    manganis::AssetSource {
                        input: #input,
                        source_file: concat!(#displayed_manifest_dir, "/", file!()),
                        local: #local,
                        bundled: #bundled,
                    }
                }
            ) #option_source
        })
    }
}

struct MethodCalls {
    options: Vec<MethodCallOption>,
}

/// A builder method in the form of `.method(arg1, arg2)`
struct MethodCallOption {
    method: syn::Ident,
    args: Punctuated<syn::Lit, Token![,]>,
}

impl MethodCalls {
    fn new(args: Vec<MethodCallOption>) -> Option<FileOptions> {
        let asset_type = args.first()?.method.to_string();

        let stack = args
            .into_iter()
            .skip(1)
            .map(|x| (x.method.to_string(), x.args.into_iter().collect::<Vec<_>>()))
            .collect::<HashMap<String, Vec<syn::Lit>>>();

        let opts = match asset_type.as_str() {
            "image" => {
                let mut opts = ImageOptions::new(manganis_common::ImageType::Avif, Some((32, 32)));
                // opts.set_preload(preload);
                // opts.set_url_encoded(url_encoded);
                // opts.set_low_quality_preview(low_quality_preview);
                FileOptions::Image(opts)
            }

            "video" => FileOptions::Video(VideoOptions::new(todo!())),
            "font" => FileOptions::Font(FontOptions::new(todo!())),
            "css" => FileOptions::Css(CssOptions::new()),
            "js" => FileOptions::Js(JsOptions::new(todo!())),
            "json" => FileOptions::Json(JsonOptions::new()),
            other => FileOptions::Other(UnknownFileOptions::new(todo!())),
        };

        Some(opts)
    }
}
