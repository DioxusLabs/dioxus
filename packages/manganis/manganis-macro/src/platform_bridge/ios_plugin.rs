// use dx_macro_helpers::linker;
use quote::{quote, ToTokens};
use std::hash::{DefaultHasher, Hash, Hasher};
use syn::{parse::Parse, parse::ParseStream, Token};

/// Parser for the `ios_plugin!()` macro syntax
pub struct IosPluginParser {
    /// Plugin identifier (e.g., "geolocation")
    plugin_name: String,

    /// Swift Package declaration
    spm: SpmDeclaration,
}

#[derive(Clone)]
struct SpmDeclaration {
    path: String,
    product: String,
}

impl Parse for IosPluginParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut plugin_name = None;
        let mut spm = None;

        while !input.is_empty() {
            let field = input.parse::<syn::Ident>()?;
            match field.to_string().as_str() {
                "plugin" => {
                    let _equals = input.parse::<Token![=]>()?;
                    let plugin_lit = input.parse::<syn::LitStr>()?;
                    plugin_name = Some(plugin_lit.value());
                    let _ = input.parse::<Option<Token![,]>>()?;
                }
                "spm" => {
                    let _equals = input.parse::<Token![=]>()?;
                    let content;
                    syn::braced!(content in input);

                    let mut path = None;
                    let mut product = None;
                    while !content.is_empty() {
                        let key = content.parse::<syn::Ident>()?;
                        let key_str = key.to_string();
                        let _eq = content.parse::<Token![=]>()?;
                        let value = content.parse::<syn::LitStr>()?;
                        match key_str.as_str() {
                            "path" => path = Some(value.value()),
                            "product" => product = Some(value.value()),
                            _ => return Err(syn::Error::new(
                                key.span(),
                                "Unknown field in spm declaration (expected 'path' or 'product')",
                            )),
                        }
                        let _ = content.parse::<Option<Token![,]>>()?;
                    }

                    let path = path.ok_or_else(|| {
                        syn::Error::new(field.span(), "Missing required field 'path' in spm block")
                    })?;
                    let product = product.ok_or_else(|| {
                        syn::Error::new(
                            field.span(),
                            "Missing required field 'product' in spm block",
                        )
                    })?;
                    spm = Some(SpmDeclaration { path, product });

                    let _ = input.parse::<Option<Token![,]>>()?;
                }
                _ => {
                    return Err(syn::Error::new(
                        field.span(),
                        "Unknown field, expected 'plugin' or 'spm'",
                    ));
                }
            }
        }

        Ok(Self {
            plugin_name: plugin_name
                .ok_or_else(|| syn::Error::new(input.span(), "Missing required field 'plugin'"))?,
            spm: spm
                .ok_or_else(|| syn::Error::new(input.span(), "Missing required field 'spm'"))?,
        })
    }
}

impl ToTokens for IosPluginParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let plugin_name = &self.plugin_name;

        let mut hash = DefaultHasher::new();
        self.plugin_name.hash(&mut hash);
        self.spm.path.hash(&mut hash);
        self.spm.product.hash(&mut hash);
        let plugin_hash = format!("{:016x}", hash.finish());

        let path_lit = syn::LitStr::new(&self.spm.path, proc_macro2::Span::call_site());
        let product_lit = syn::LitStr::new(&self.spm.product, proc_macro2::Span::call_site());

        let metadata_expr = quote! {
            manganis::darwin::SwiftSourceMetadata::new(
                #plugin_name,
                concat!(env!("CARGO_MANIFEST_DIR"), "/", #path_lit),
                #product_lit,
            )
        };

        let link_section = crate::permissions::generate_link_section_inner(
            metadata_expr,
            &plugin_hash,
            "__ASSETS__",
            quote! { manganis::darwin::metadata::serialize_swift_metadata },
            quote! { manganis::darwin::macro_helpers::copy_bytes },
            quote! { manganis::darwin::metadata::SwiftMetadataBuffer },
            true,
        );

        tokens.extend(link_section);
    }
}
