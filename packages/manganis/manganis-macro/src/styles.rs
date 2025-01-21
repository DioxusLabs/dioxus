use crate::{asset::AssetParser, resolve_path};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    token::Comma,
    Ident, LitStr,
};

pub(crate) struct StyleParser {
    asset_ident: Ident,
    styles_ident: Ident,
    asset_parser: AssetParser,
}

impl Parse for StyleParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let asset_ident = input.parse::<Ident>()?;
        input.parse::<Comma>()?;
        let styles_ident = input.parse::<Ident>()?;
        input.parse::<Comma>()?;
        let src = input.parse::<LitStr>()?;
        let path_span = src.span();
        let asset = resolve_path(&src.value());
        let _comma = input.parse::<Comma>();

        let mut options = input.parse::<TokenStream>()?;
        if options.is_empty() {
            options = quote! { manganis::CssModuleAssetOptions::new() }
        }
        // TODO: Verify that this is actually a `CssModuleAssetOptions`

        let asset_parser = AssetParser {
            path_span,
            asset,
            options,
        };

        Ok(Self { asset_ident, styles_ident, asset_parser })
    }
}

impl ToTokens for StyleParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        // Use the regular asset parser to generate the linker bridge.
        let asset_ident = &self.asset_ident;
        let mut linker_tokens = quote! { const #asset_ident: manganis::Asset = };
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
        let (classes, ids, _) = manganis_core::collect_css_idents(&css);

        let mut fields = Vec::new();
        let mut values = Vec::new();

        for id in ids.iter() {
            let as_snake = to_snake_case(id);
            let ident = Ident::new(&as_snake, Span::call_site());

            fields.push(quote! {
                pub #ident: &'a str,
            });

            values.push(quote! {
                #ident: manganis::macro_helpers::const_serialize::ConstStr::new(#id).push_str(__styles::__ASSET_HASH.as_str()).as_str(),
            });
        }

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
                #ident: manganis::macro_helpers::const_serialize::ConstStr::new(#class).push_str(__styles::__ASSET_HASH.as_str()).as_str(),
            });
        }

        if fields.is_empty() {
            panic!("NO CSS IDENTS!");
        }

        let styles_ident = &self.styles_ident;
        let options = &self.asset_parser.options;

        tokens.extend(quote! {
            #[doc(hidden)]
            mod __styles {
                use super::manganis;

                const __ASSET_OPTIONS: manganis::AssetOptions = #options.into_asset_options();
                pub(super) const __ASSET_HASH: manganis::macro_helpers::const_serialize::ConstStr = manganis::macro_helpers::hash_asset(&__ASSET_OPTIONS, #hash);

                pub(super) struct Styles<'a> {
                    #( #fields )*
                }
            }

            const #styles_ident: __styles::Styles = __styles::Styles {
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
