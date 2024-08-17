use manganis_common::{AssetType, FileOptions, ManganisSupportError, ResourceAsset};
use quote::{quote, ToTokens};
use syn::{parenthesized, parse::Parse};

use crate::{generate_link_section, resource::ResourceAssetParser};

pub enum ParseJsonOption {
    UrlEncoded(bool),
    Preload(bool),
}

impl ParseJsonOption {
    fn apply(options: Vec<Self>, file: &mut ResourceAsset) {
        for option in options {
            option.apply_to_options(file);
        }
    }
    fn apply_to_options(self, file: &mut ResourceAsset) {
        match self {
            ParseJsonOption::Preload(preload) => file.with_options_mut(|options| {
                if let FileOptions::Json(options) = options {
                    options.set_preload(preload);
                }
            }),
            ParseJsonOption::UrlEncoded(url_encoded) => {
                file.set_url_encoded(url_encoded);
            }
        }
    }
}

impl Parse for ParseJsonOption {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<syn::Token![.]>()?;
        let ident = input.parse::<syn::Ident>()?;
        let _content;
        parenthesized!(_content in input);
        match ident.to_string().as_str() {
            "preload" => {
                crate::verify_preload_valid(&ident)?;
                Ok(ParseJsonOption::Preload(true))
            },
            "url_encoded" => Ok(ParseJsonOption::UrlEncoded(true)),
            _ => Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "Unknown Json option: {}. Supported options are preload, url_encoded, and minify",
                    ident
                ),
            )),
        }
    }
}

// let file_name = if asset.url_encoded() {
//     #[cfg(not(feature = "url-encoding"))]
//     return Err(syn::Error::new(
//         proc_macro2::Span::call_site(),
//         "URL encoding is not enabled. Enable the url-encoding feature to use this feature",
//     ));
//     #[cfg(feature = "url-encoding")]
//     Ok(crate::url_encoded_asset(&asset).map_err(|e| {
//         syn::Error::new(
//             proc_macro2::Span::call_site(),
//             format!("Failed to encode file: {}", e),
//         )
//     })?)
// } else {
//     asset.served_location()
// };
