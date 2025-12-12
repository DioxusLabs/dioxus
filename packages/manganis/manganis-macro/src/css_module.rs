use crate::{asset::AssetParser, resolve_path};
use macro_string::MacroString;
use manganis_core::{create_module_hash, get_class_mappings};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::Comma,
    Ident,
};

pub(crate) struct CssModuleParser {
    asset_parser: AssetParser,
}

impl Parse for CssModuleParser {
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

impl ToTokens for CssModuleParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        // Use the regular asset parser to generate the linker bridge.
        let mut linker_tokens = quote! {
            /// Auto-generated Manganis asset for css modules.
            #[allow(missing_docs)]
            const ASSET: manganis::Asset =
        };
        self.asset_parser.to_tokens(&mut linker_tokens);

        let asset = match self.asset_parser.asset.as_ref() {
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
                pub const #ident: __Styles::__CssIdent = __Styles::__CssIdent { inner: #new_class };
            });
        }

        // We use a PhantomData to prevent Rust from complaining about an unused lifetime if a css module without any idents is used.
        tokens.extend(quote! {
            #[doc(hidden)]
            #[allow(missing_docs, non_snake_case)]
            mod __Styles {
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
            pub struct Styles {}

            impl Styles {
                #( #values )*
            }

                impl dioxus::core::IntoAttributeValue for __Styles::__CssIdent {
                    fn into_value(self) -> dioxus::core::AttributeValue {
                        dioxus::core::AttributeValue::Text(self.to_string())
                    }
                }
        })
    }
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
