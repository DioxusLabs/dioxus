use crate::{asset::AssetParser, resolve_path};
use macro_string::MacroString;
use manganis_core::{create_module_hash, get_class_mappings};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::Comma,
    Ident, ItemStruct,
};

pub(crate) struct CssModuleAttribute {
    asset_parser: AssetParser,
}

impl Parse for CssModuleAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Asset path "/path.css"
        let (MacroString(src), path_expr) = input.call(crate::parse_with_tokens)?;
        let asset = resolve_path(&src, path_expr.span());

        let _comma = input.parse::<Comma>();

        // Optional options
        let mut options = input.parse::<TokenStream>()?;
        if options.is_empty() {
            options = quote! { manganis::AssetOptions::css_module() }
        }

        let asset_parser = AssetParser {
            path_expr,
            asset,
            options,
        };

        Ok(Self { asset_parser })
    }
}

pub(crate) fn expand_css_module_struct(
    tokens: &mut proc_macro2::TokenStream,
    attribute: &CssModuleAttribute,
    item_struct: &ItemStruct,
) {
    if !item_struct.fields.is_empty() {
        let err = syn::Error::new(
            item_struct.fields.span(),
            "css_module can only be applied to unit structs",
        )
        .into_compile_error();
        tokens.append_all(err);
        return;
    }
    if !item_struct.generics.params.is_empty() {
        let err = syn::Error::new(
            item_struct.generics.span(),
            "css_module cannot be applied to generic structs",
        )
        .into_compile_error();
        tokens.append_all(err);
        return;
    }
    let struct_vis = &item_struct.vis;
    let struct_name = &item_struct.ident;
    let struct_name_private = format_ident!("__{}", struct_name);

    // Use the regular asset parser to generate the linker bridge.
    let mut linker_tokens = quote! {
        /// Auto-generated Manganis asset for css modules.
        #[allow(missing_docs)]
        const ASSET: manganis::Asset =
    };
    attribute.asset_parser.to_tokens(&mut linker_tokens);

    let asset = match attribute.asset_parser.asset.as_ref() {
        Ok(path) => path,
        Err(err) => {
            let err = err.to_string();
            tokens.append_all(quote! { compile_error!(#err) });
            return;
        }
    };

    let css = std::fs::read_to_string(asset).expect("Unable to read css module file");

    let mut values = Vec::new();

    let hash = create_module_hash(asset);
    let class_mappings = get_class_mappings(css.as_str(), hash.as_str()).expect("Invalid css");

    // Generate class struct field tokens.
    for (old_class, new_class) in class_mappings.iter() {
        let as_snake = to_snake_case(old_class);

        let ident = Ident::new(&as_snake, Span::call_site());
        values.push(quote! {
            pub const #ident: #struct_name_private::__CssIdent = #struct_name_private::__CssIdent { inner: #new_class };
        });
    }

    // We use a PhantomData to prevent Rust from complaining about an unused lifetime if a css module without any idents is used.
    tokens.extend(quote! {
        #[doc(hidden)]
        #[allow(missing_docs, non_snake_case)]
        mod #struct_name_private {
            use dioxus::prelude::*;

            #linker_tokens;

            // Css ident class for deref stylesheet inclusion.
            pub struct __CssIdent { pub inner: &'static str }

            use std::ops::Deref;
            use std::sync::OnceLock;
            use dioxus::document::{document, LinkProps};

            impl Deref for __CssIdent {
                type Target = str;

                fn deref(&self) -> &Self::Target {
                    static CELL: OnceLock<()> = OnceLock::new();
                    CELL.get_or_init(move || {
                        let doc = document();
                        doc.create_link(
                            LinkProps::builder()
                                .rel(Some("stylesheet".to_string()))
                                .href(Some(ASSET.to_string()))
                                .build(),
                        );
                    });

                    self.inner
                }
            }

            impl std::fmt::Display for __CssIdent {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                    self.deref().fmt(f)
                }
            }
        }

        /// Auto-generated idents struct for CSS modules.
        #[allow(missing_docs, non_snake_case)]
        #struct_vis struct #struct_name {}

        impl #struct_name {
            #( #values )*
        }

            impl dioxus::core::IntoAttributeValue for #struct_name_private::__CssIdent {
                fn into_value(self) -> dioxus::core::AttributeValue {
                    dioxus::core::AttributeValue::Text(self.to_string())
                }
            }
    })
}

/// Convert camel and kebab case to snake case.
///
/// This can fail sometimes, for example `myCss-Class`` is `my_css__class`
fn to_snake_case(input: &str) -> String {
    let mut new = String::new();

    for (i, c) in input.chars().enumerate() {
        if c.is_uppercase() && i != 0 {
            new.push('_');
        }

        new.push(c.to_ascii_lowercase());
    }

    new.replace('-', "_")
}
