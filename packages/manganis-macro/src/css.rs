use manganis_common::{AssetType, CssOptions, FileOptions, ManganisSupportError, ResourceAsset};
use quote::{quote, ToTokens};
use syn::{parenthesized, parse::Parse, LitBool};

// use crate::{generate_link_section, resource::ResourceAssetParser};

struct ParseCssOptions {
    options: Vec<ParseCssOption>,
}

impl ParseCssOptions {
    fn apply_to_options(self, file: &mut ResourceAsset) {
        for option in self.options {
            option.apply_to_options(file);
        }
    }
}

impl Parse for ParseCssOptions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut options = Vec::new();
        while !input.is_empty() {
            options.push(input.parse::<ParseCssOption>()?);
        }
        Ok(ParseCssOptions { options })
    }
}

enum ParseCssOption {
    UrlEncoded(bool),
    Preload(bool),
    Minify(bool),
}

impl ParseCssOption {
    fn apply_to_options(self, file: &mut ResourceAsset) {
        match self {
            ParseCssOption::Preload(_) | ParseCssOption::Minify(_) => {
                file.with_options_mut(|options| {
                    if let FileOptions::Css(options) = options {
                        match self {
                            ParseCssOption::Minify(format) => {
                                options.set_minify(format);
                            }
                            ParseCssOption::Preload(preload) => {
                                options.set_preload(preload);
                            }
                            _ => {}
                        }
                    }
                })
            }
            ParseCssOption::UrlEncoded(url_encoded) => {
                file.set_url_encoded(url_encoded);
            }
        }
    }
}

impl Parse for ParseCssOption {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<syn::Token![.]>()?;
        let ident = input.parse::<syn::Ident>()?;
        let content;
        parenthesized!(content in input);
        match ident.to_string().as_str() {
            "preload" => {
                crate::verify_preload_valid(&ident)?;
                Ok(ParseCssOption::Preload(true))
            }
            "url_encoded" => {
                Ok(ParseCssOption::UrlEncoded(true))
            }
            "minify" => {
                Ok(ParseCssOption::Minify(content.parse::<LitBool>()?.value()))
            }
            _ => Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "Unknown Css option: {}. Supported options are preload, url_encoded, and minify",
                    ident
                ),
            )),
        }
    }
}

pub struct CssAssetParser {
    asset: ResourceAsset,
}

impl Parse for CssAssetParser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inside;
        parenthesized!(inside in input);
        let path = inside.parse::<syn::LitStr>()?;

        let parsed_options = {
            if input.is_empty() {
                None
            } else {
                Some(input.parse::<ParseCssOptions>()?)
            }
        };

        let path_as_str = path.value();

        let mut asset: ResourceAsset = match ResourceAsset::parse_file(&path_as_str) {
            Ok(asset) => asset.with_options(manganis_common::FileOptions::Css(CssOptions::new())),
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

        Ok(CssAssetParser { asset })
    }
}

impl ToTokens for CssAssetParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        ResourceAssetParser::to_ref_tokens(&self.asset, tokens)
    }
}
