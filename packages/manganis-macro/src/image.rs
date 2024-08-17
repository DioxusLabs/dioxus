use manganis_common::ManganisSupportError;
use manganis_common::{AssetType, FileOptions, ImageOptions, ResourceAsset};
use quote::{quote, ToTokens};
use syn::{parenthesized, parse::Parse, Token};

use crate::generate_link_section;

struct ParseImageOptions {
    options: Vec<ParseImageOption>,
}

impl ParseImageOptions {
    fn apply_to_options(self, file: &mut ResourceAsset, low_quality_preview: &mut bool) {
        for option in self.options {
            option.apply_to_options(file, low_quality_preview);
        }
    }
}

impl Parse for ParseImageOptions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut options = Vec::new();
        while !input.is_empty() {
            options.push(input.parse::<ParseImageOption>()?);
        }
        Ok(ParseImageOptions { options })
    }
}

enum ParseImageOption {
    Format(manganis_common::ImageType),
    Size((u32, u32)),
    Preload(bool),
    UrlEncoded(bool),
    Lqip(bool),
}

impl ParseImageOption {
    fn apply_to_options(self, file: &mut ResourceAsset, low_quality_preview: &mut bool) {
        match self {
            ParseImageOption::Format(_)
            | ParseImageOption::Size(_)
            | ParseImageOption::Preload(_) => file.with_options_mut(|options| {
                if let FileOptions::Image(options) = options {
                    match self {
                        ParseImageOption::Format(format) => {
                            options.set_ty(format);
                        }
                        ParseImageOption::Size(size) => {
                            options.set_size(Some(size));
                        }
                        ParseImageOption::Preload(preload) => {
                            options.set_preload(preload);
                        }
                        _ => {}
                    }
                }
            }),
            ParseImageOption::UrlEncoded(url_encoded) => {
                file.set_url_encoded(url_encoded);
            }
            ParseImageOption::Lqip(lqip) => {
                *low_quality_preview = lqip;
            }
        }
    }
}

impl Parse for ParseImageOption {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<syn::Token![.]>()?;
        let ident = input.parse::<syn::Ident>()?;
        let content;
        parenthesized!(content in input);
        match ident.to_string().as_str() {
            "format" => {
                let format = content.parse::<ImageType>()?;
                Ok(ParseImageOption::Format(format.into()))
            }
            "size" => {
                let size = content.parse::<ImageSize>()?;
                Ok(ParseImageOption::Size((size.width, size.height)))
            }
            "preload" => {
                crate::verify_preload_valid(&ident)?;
                Ok(ParseImageOption::Preload(true))
            }
            "url_encoded" => {
                Ok(ParseImageOption::UrlEncoded(true))
            }
            "low_quality_preview" => {
                Ok(ParseImageOption::Lqip(true))
            }
            _ => Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "Unknown image option: {}. Supported options are format, size, preload, url_encoded, low_quality_preview",
                    ident
                ),
            )),
        }
    }
}

struct ImageSize {
    width: u32,
    height: u32,
}

impl Parse for ImageSize {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let width = input.parse::<syn::LitInt>()?;
        let _ = input.parse::<syn::Token![,]>()?;
        let height = input.parse::<syn::LitInt>()?;
        Ok(ImageSize {
            width: width.base10_parse()?,
            height: height.base10_parse()?,
        })
    }
}

impl From<ImageType> for manganis_common::ImageType {
    fn from(val: ImageType) -> Self {
        val.0
    }
}

impl Parse for ImageType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<syn::Ident>()?;
        let _ = input.parse::<Token![::]>()?;
        let ident = input.parse::<syn::Ident>()?;
        ident
            .to_string()
            .to_lowercase()
            .as_str()
            .parse::<manganis_common::ImageType>()
            .map_err(|_| {
                syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!(
                        "Unknown image type: {}. Supported types are png, jpeg, webp, avif",
                        ident
                    ),
                )
            })
            .map(Self)
    }
}

#[derive(Clone, Copy)]
struct ImageType(manganis_common::ImageType);

impl Default for ImageType {
    fn default() -> Self {
        Self(manganis_common::ImageType::Avif)
    }
}

pub struct ImageAssetParser {
    asset: ResourceAsset,
    low_quality_preview: Option<String>,
}

impl Parse for ImageAssetParser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inside;
        parenthesized!(inside in input);
        let path = inside.parse::<syn::LitStr>()?;

        let parsed_options = {
            if input.is_empty() {
                None
            } else {
                Some(input.parse::<ParseImageOptions>()?)
            }
        };

        let path_as_str = path.value();
        let mut asset: ResourceAsset = match ResourceAsset::parse_file(&path_as_str) {
            Ok(path) => path.with_options(manganis_common::FileOptions::Image(ImageOptions::new(
                manganis_common::ImageType::Avif,
                None,
            ))),
            Err(e) => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("{e}"),
                ))
            }
        };

        let mut low_quality_preview = false;
        if let Some(parsed_options) = parsed_options {
            parsed_options.apply_to_options(&mut asset, &mut low_quality_preview);
        }

        // let asset = manganis_common::AssetType::Resource(asset.clone());

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

        // let low_quality_preview = if low_quality_preview {
        //     #[cfg(not(feature = "url-encoding"))]
        //     return Err(syn::Error::new(
        //         proc_macro2::Span::call_site(),
        //         "Low quality previews require URL encoding. Enable the url-encoding feature to use this feature",
        //     ));

        //     #[cfg(feature = "url-encoding")]
        //     {
        //         let current_image_size = match asset.options() {
        //             manganis_common::FileOptions::Image(options) => options.size(),
        //             _ => None,
        //         };
        //         let low_quality_preview_size = current_image_size
        //             .map(|(width, height)| {
        //                 let width = width / 10;
        //                 let height = height / 10;
        //                 (width, height)
        //             })
        //             .unwrap_or((32, 32));
        //         let lqip = ResourceAsset::new(asset).with_options(
        //             manganis_common::FileOptions::Image(ImageOptions::new(
        //                 manganis_common::ImageType::Avif,
        //                 Some(low_quality_preview_size),
        //             )),
        //         );

        //         Some(crate::url_encoded_asset(&lqip).map_err(|e| {
        //             syn::Error::new(
        //                 proc_macro2::Span::call_site(),
        //                 format!("Failed to encode file: {}", e),
        //             )
        //         })?)
        //     }
        // } else {
        //     None
        // };

        let low_quality_preview = None;

        Ok(ImageAssetParser {
            low_quality_preview,
            asset,
        })
    }
}

impl ToTokens for ImageAssetParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let link_section = generate_link_section(&self.asset);
        let input = self.asset.input.to_string();

        let bundled = self.asset.bundled.to_string();

        let low_quality_preview = match &self.low_quality_preview {
            Some(lqip) => quote! { Some(#lqip) },
            None => quote! { None },
        };

        // If the asset is relative, we use concat!(env!("CARGO_MANIFEST_DIR"), "/", asset.input.path())
        let local = match self.asset.local.as_ref() {
            Some(local) => {
                let local = local.to_string();
                quote! { #local }
            }
            None => {
                quote! {
                    {
                        static _: &[_] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #input.path()));
                        concat!(env!("CARGO_MANIFEST_DIR"), "/", #input.path())
                    }
                }
            }
        };

        tokens.extend(quote! {
            {
                #link_section
                manganis::ImageAsset::new(
                    manganis::Asset {
                        input: #input,
                        local: #local,
                        bundled: #bundled,
                    }
                ).with_preview(#low_quality_preview)
            }
        })
    }
}
