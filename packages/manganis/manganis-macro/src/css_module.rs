use crate::{asset::AssetParser, resolve_path};
use macro_string::MacroString;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    token::Comma,
    Ident, Visibility,
};

pub(crate) struct CssModuleParser {
    asset_vis: Visibility,
    asset_ident: Ident,
    styles_vis: Visibility,
    styles_ident: Ident,
    asset_parser: AssetParser,
}

impl Parse for CssModuleParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // macro!(pub? ASSET_IDENT, pub? STYLES_IDENT, ASSET_PATH, ASSET_OPTIONS?)
        // Asset struct ident
        let asset_vis = input.parse::<Visibility>()?;
        let asset_ident = input.parse::<Ident>()?;
        input.parse::<Comma>()?;

        // Styles struct ident
        let styles_vis = input.parse::<Visibility>()?;
        let styles_ident = input.parse::<Ident>()?;
        input.parse::<Comma>()?;

        // Asset path
        let (MacroString(src), path_expr) = input.call(crate::parse_with_tokens)?;
        let asset = resolve_path(&src);
        let _comma = input.parse::<Comma>();

        // Optional options
        let mut options = input.parse::<TokenStream>()?;
        if options.is_empty() {
            options = quote! { manganis::CssModuleAssetOptions::new() }
        }

        let asset_parser = AssetParser {
            path_expr,
            asset,
            options,
        };

        Ok(Self {
            asset_vis,
            asset_ident,
            styles_vis,
            styles_ident,
            asset_parser,
        })
    }
}

impl ToTokens for CssModuleParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        // Use the regular asset parser to generate the linker bridge.
        let asset_vis = &self.asset_vis;
        let asset_ident = &self.asset_ident;
        let mut linker_tokens = quote! {
            /// Auto-generated Manganis asset for css modules.
            #[allow(missing_docs)]
            #asset_vis const #asset_ident: manganis::Asset =
        };
        self.asset_parser.to_tokens(&mut linker_tokens);
        tokens.extend(quote! { #linker_tokens; });

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

        let mut fields = Vec::new();
        let mut values = Vec::new();

        // Create unique module name based on styles ident.
        let styles_ident = &self.styles_ident;
        let mod_name = format_ident!("__{}_module", styles_ident);

        // Generate id struct field tokens.
        for id in ids.iter() {
            let as_snake = to_snake_case(id);
            let ident = Ident::new(&as_snake, Span::call_site());

            fields.push(quote! {
                pub #ident: &'a str,
            });

            values.push(quote! {
                #ident: manganis::macro_helpers::const_serialize::ConstStr::new(#id).push_str(#mod_name::__ASSET_HASH.as_str()).as_str(),
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
            fields.push(quote! {
                pub #ident: &'a str,
            });

            values.push(quote! {
                #ident: manganis::macro_helpers::const_serialize::ConstStr::new(#class).push_str(#mod_name::__ASSET_HASH.as_str()).as_str(),
            });
        }

        let options = &self.asset_parser.options;
        let styles_vis = &self.styles_vis;

        // We use a PhantomData to prevent Rust from complaining about an unused lifetime if a css module without any idents is used.
        tokens.extend(quote! {
            #[doc(hidden)]
            #[allow(missing_docs, non_snake_case)]
            mod #mod_name {
                use super::manganis;

                const __ASSET_OPTIONS: manganis::AssetOptions = #options.into_asset_options();
                pub(super) const __ASSET_HASH: manganis::macro_helpers::const_serialize::ConstStr = manganis::macro_helpers::hash_asset(&__ASSET_OPTIONS, #hash);

                pub(super) struct Styles<'a> {
                    pub __phantom: std::marker::PhantomData<&'a ()>,
                    #( #fields )*
                }
            }

            /// Auto-generated idents struct for CSS modules.
            #[allow(missing_docs, non_snake_case)]
            #styles_vis const #styles_ident: #mod_name::Styles = #mod_name::Styles {
                __phantom: std::marker::PhantomData,
                #( #values )*
            };
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
