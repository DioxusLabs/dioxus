use crate::{asset::AssetParser, resolve_path};
use macro_string::MacroString;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    token::Comma,
    Ident, Token, Visibility,
};

pub(crate) struct CssModuleParser {
    /// Whether the ident is const or static.
    styles_vis: Visibility,
    styles_ident: Ident,
    asset_parser: AssetParser,
}

impl Parse for CssModuleParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // NEW: macro!(pub? STYLES_IDENT = "/path.css");
        // pub(x)?
        let styles_vis = input.parse::<Visibility>()?;

        // Styles Ident
        let styles_ident = input.parse::<Ident>()?;
        let _equals = input.parse::<Token![=]>()?;

        // Asset path "/path.css"
        let (MacroString(src), path_expr) = input.call(crate::parse_with_tokens)?;
        let asset = resolve_path(&src);

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

        Ok(Self {
            styles_vis,
            styles_ident,
            asset_parser,
        })
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

        let path = match self.asset_parser.asset.as_ref() {
            Ok(path) => path,
            Err(err) => {
                let err = err.to_string();
                tokens.append_all(quote! { compile_error!(#err) });
                return;
            }
        };

        // Get the file hash
        let hash = match crate::hash_file_contents(path) {
            Ok(hash) => hash,
            Err(err) => {
                let err = err.to_string();
                tokens.append_all(quote! { compile_error!(#err) });
                return;
            }
        };

        // Process css idents
        let css = std::fs::read_to_string(path).unwrap();
        let (classes, ids) = manganis_core::collect_css_idents(&css);

        let mut values = Vec::new();

        // Create unique module name based on styles ident.
        let styles_ident = &self.styles_ident;
        let mod_name = format_ident!("__{}_module", styles_ident);

        // Generate id struct field tokens.
        for id in ids.iter() {
            let as_snake = to_snake_case(id);
            let ident = Ident::new(&as_snake, Span::call_site());

            values.push(quote! {
                pub const #ident: #mod_name::__CssIdent = #mod_name::__CssIdent { inner: manganis::macro_helpers::const_serialize::ConstStr::new(#id).push_str(#mod_name::__ASSET_HASH.as_str()).as_str() };
            });
        }

        // Generate class struct field tokens.
        for class in classes.iter() {
            let as_snake = to_snake_case(class);
            let as_snake = match ids.contains(class) {
                false => as_snake,
                true => format!("{as_snake}_class"),
            };

            let ident = Ident::new(&as_snake, Span::call_site());
            values.push(quote! {
                pub const #ident: #mod_name::__CssIdent = #mod_name::__CssIdent { inner: manganis::macro_helpers::const_serialize::ConstStr::new(#class).push_str(#mod_name::__ASSET_HASH.as_str()).as_str() };
            });
        }

        let options = &self.asset_parser.options;
        let styles_vis = &self.styles_vis;

        // We use a PhantomData to prevent Rust from complaining about an unused lifetime if a css module without any idents is used.
        tokens.extend(quote! {
            #[doc(hidden)]
            #[allow(missing_docs, non_snake_case)]
            mod #mod_name {
                #[allow(unused_imports)]
                use manganis::{self, CssModuleAssetOptions};

                #linker_tokens;

                // Get the hash to use when builidng hashed css idents.
                const __ASSET_OPTIONS: manganis::AssetOptions = #options.into_asset_options();
                pub(super) const __ASSET_HASH: manganis::macro_helpers::const_serialize::ConstStr = manganis::macro_helpers::hash_asset(&__ASSET_OPTIONS, #hash);

                // Css ident class for deref stylesheet inclusion.
                pub(super) struct __CssIdent { pub inner: &'static str }

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
            #styles_vis struct #styles_ident {}

            impl #styles_ident {
                #( #values )*
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
