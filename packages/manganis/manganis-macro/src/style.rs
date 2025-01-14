use crate::{resolve_path, AssetParseError};
use proc_macro2::Span;
use quote::{quote, ToTokens, TokenStreamExt};
use std::collections::HashSet;
use std::path::PathBuf;
use syn::{
    parse::{Parse, ParseStream},
    token::Comma,
    Ident, LitStr,
};

pub(crate) struct StyleParser {
    user_ident: Ident,
    path: Result<PathBuf, AssetParseError>,
}

impl Parse for StyleParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let user_ident = input.parse::<Ident>()?;
        input.parse::<Comma>()?;
        let path = input.parse::<LitStr>()?;

        let path = resolve_path(&path.value());

        Ok(Self { user_ident, path })
    }
}

impl ToTokens for StyleParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let path = match self.path.as_ref() {
            Ok(path) => path,
            Err(err) => {
                let err = err.to_string();
                tokens.append_all(quote! { compile_error!(#err) });
                return;
            }
        };

        let css = std::fs::read_to_string(path).unwrap();
        let (classes, ids) = collect_css_idents(&css);

        let mut fields = Vec::new();
        let mut values = Vec::new();

        for id in ids.iter() {
            let as_snake = to_snake_case(id);
            let ident = Ident::new(&as_snake, Span::call_site());

            fields.push(quote! {
                pub #ident: &'a str,
            });

            values.push(quote! {
                #ident: #id,
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
                #ident: #class,
            });
        }

        if fields.is_empty() {
            panic!("NO CSS IDENTS!");
        }

        let user_ident = &self.user_ident;
        tokens.extend(quote! {
            #[doc(hidden)]
            mod __styles {
                pub struct Styles<'a> {
                    #( #fields )*
                }
            }

            const #user_ident: __styles::Styles = __styles::Styles {
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

// TODO: Make sure we're not taking tokens from inside comments.
/// Collect CSS classes & ids
///
/// Returns (HashSet<Classes>, HashSet<Ids>)
fn collect_css_idents(css: &str) -> (HashSet<String>, HashSet<String>) {
    const ALLOWED: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-";

    let mut classes = HashSet::new();
    let mut ids = HashSet::new();

    // Collected ident name and true if it's an id.
    let mut start: Option<(String, bool)> = None;

    // If we are currently collecting an ident:
    // - Check if the char is allowed, put it into the ident string.
    // - If not allowed, finalize the ident string and reset start.
    // Otherwise:
    // Check if character is a `.` or `#` representing a class or string,
    // and start collecting.
    for c in css.chars() {
        if let Some(ident) = start.as_mut() {
            if ALLOWED.find(c).is_some() {
                // CSS ignore idents that start with a number.
                // 1. Difficult to process
                // 2. Avoid false positives (transition: 0.5s)
                if ident.0.is_empty() && c.is_numeric() {
                    start = None;
                    continue;
                }

                ident.0.push(c);
            } else {
                match ident.1 {
                    true => ids.insert(ident.0.clone()),
                    false => classes.insert(ident.0.clone()),
                };

                start = None;
            }
        } else {
            if c == '.' {
                start = Some((String::new(), false));
            } else if c == '#' {
                start = Some((String::new(), true));
            }
        }
    }

    (classes, ids)
}
