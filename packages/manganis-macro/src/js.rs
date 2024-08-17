use manganis_common::{
    AssetType, FileOptions, JsOptions, JsType, ManganisSupportError, ResourceAsset,
};
use quote::{quote, ToTokens};
use syn::{parenthesized, parse::Parse, LitBool};

use crate::{generate_link_section, resource::ResourceAssetParser};

struct ParseJsOptions {
    options: Vec<ParseJsOption>,
}

impl ParseJsOptions {
    fn apply_to_options(self, file: &mut ResourceAsset) {
        for option in self.options {
            option.apply_to_options(file);
        }
    }
}

impl Parse for ParseJsOptions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut options = Vec::new();
        while !input.is_empty() {
            options.push(input.parse::<ParseJsOption>()?);
        }
        Ok(ParseJsOptions { options })
    }
}

enum ParseJsOption {
    UrlEncoded(bool),
    Preload(bool),
    Minify(bool),
}

impl ParseJsOption {
    fn apply_to_options(self, file: &mut ResourceAsset) {
        match self {
            ParseJsOption::Preload(_) | ParseJsOption::Minify(_) => {
                file.with_options_mut(|options| {
                    if let FileOptions::Js(options) = options {
                        match self {
                            ParseJsOption::Minify(format) => {
                                options.set_minify(format);
                            }
                            ParseJsOption::Preload(preload) => {
                                options.set_preload(preload);
                            }
                            _ => {}
                        }
                    }
                })
            }
            ParseJsOption::UrlEncoded(url_encoded) => {
                file.set_url_encoded(url_encoded);
            }
        }
    }
}

impl Parse for ParseJsOption {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<syn::Token![.]>()?;
        let ident = input.parse::<syn::Ident>()?;
        let content;
        parenthesized!(content in input);
        match ident.to_string().as_str() {
            "preload" => {
                crate::verify_preload_valid(&ident)?;
                Ok(ParseJsOption::Preload(true))
            }
            "url_encoded" => Ok(ParseJsOption::UrlEncoded(true)),
            "minify" => Ok(ParseJsOption::Minify(content.parse::<LitBool>()?.value())),
            _ => Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "Unknown Js option: {}. Supported options are preload, url_encoded, and minify",
                    ident
                ),
            )),
        }
    }
}

pub struct JsAssetParser {
    asset: ResourceAsset,
}

impl Parse for JsAssetParser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inside;
        parenthesized!(inside in input);
        let path = inside.parse::<syn::LitStr>()?;

        let parsed_options = {
            if input.is_empty() {
                None
            } else {
                Some(input.parse::<ParseJsOptions>()?)
            }
        };

        let path_as_str = path.value();
        let mut asset: ResourceAsset = match ResourceAsset::parse_file(&path_as_str) {
            Ok(path) => {
                path.with_options(manganis_common::FileOptions::Js(JsOptions::new(JsType::Js)))
            }
            Err(e) => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("{e}"),
                ))
            }
        };

        if let Some(parsed_options) = parsed_options {
            parsed_options.apply_to_options(&mut asset);
        }

        // let mut this_file = ResourceAsset::new(path.clone())
        //     .with_options(manganis_common::FileOptions::Js(JsOptions::new(JsType::Js)));
        // let asset = manganis_common::AssetType::Resource(this_file.clone());

        // let file_name = if this_file.url_encoded() {
        //     #[cfg(not(feature = "url-encoding"))]
        //     return Err(syn::Error::new(
        //         proc_macro2::Span::call_site(),
        //         "URL encoding is not enabled. Enable the url-encoding feature to use this feature",
        //     ));
        //     #[cfg(feature = "url-encoding")]
        //     Ok(crate::url_encoded_asset(&this_file).map_err(|e| {
        //         syn::Error::new(
        //             proc_macro2::Span::call_site(),
        //             format!("Failed to encode file: {}", e),
        //         )
        //     })?)
        // } else {
        //     this_file.served_location()
        // };

        Ok(JsAssetParser { asset })
    }
}

impl ToTokens for JsAssetParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        ResourceAssetParser::to_ref_tokens(&self.asset, tokens)
    }
}
