use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Error, Ident, Result, Token,
};

use super::*;

pub struct RsxBody<const AS: HTML_OR_RSX> {
    custom_context: Option<Ident>,
    roots: Vec<BodyNode<AS>>,
}

/// The custom rusty variant of parsing rsx!
impl Parse for RsxBody<AS_RSX> {
    fn parse(input: ParseStream) -> Result<Self> {
        let custom_context = try_parse_custom_context(input)?;
        let (_, roots, _) =
            BodyParseConfig::<AS_RSX>::new_as_body().parse_component_body(&input)?;
        Ok(Self {
            custom_context,
            roots,
        })
    }
}

/// The HTML variant of parsing rsx!
impl Parse for RsxBody<AS_HTML> {
    fn parse(input: ParseStream) -> Result<Self> {
        let custom_context = try_parse_custom_context(input)?;
        let (_, roots, _) =
            BodyParseConfig::<AS_HTML>::new_as_body().parse_component_body(&input)?;
        Ok(Self {
            custom_context,
            roots,
        })
    }
}

fn try_parse_custom_context(input: ParseStream) -> Result<Option<Ident>> {
    let res = if input.peek(Ident) && input.peek2(Token![,]) {
        let name = input.parse::<Ident>()?;
        input.parse::<Token![,]>()?;
        Some(name)
    } else {
        None
    };
    Ok(res)
}

/// Serialize the same way, regardless of flavor
impl<const A: HTML_OR_RSX> ToTokens for RsxBody<A> {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let inner = if self.roots.len() == 1 {
            let inner = &self.roots[0];
            quote! {#inner}
        } else {
            let childs = &self.roots;
            quote! { __cx.fragment_from_iter([ #(#childs),* ]) }
        };

        match &self.custom_context {
            // The `in cx` pattern allows directly rendering
            Some(ident) => out_tokens.append_all(quote! {
                #ident.render(dioxus::prelude::LazyNodes::new(move |__cx: NodeFactory|{
                    use dioxus_elements::{GlobalAttributes, SvgAttributes};

                    #inner
                }))
            }),
            // Otherwise we just build the LazyNode wrapper
            None => out_tokens.append_all(quote! {
                dioxus::prelude::LazyNodes::new(move |__cx: NodeFactory|{
                    use dioxus_elements::{GlobalAttributes, SvgAttributes};

                    #inner
                 })
            }),
        };
    }
}
