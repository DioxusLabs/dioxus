use crate::util::is_valid_tag;
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
        // if input.peek(LitStr) {
        //     return input.parse::<LitStr>()?.parse::<RsxRender>();
        // }

        // try to parse the first ident and comma
        let custom_context =
            if input.peek(Token![in]) && input.peek2(Ident) && input.peek3(Token![,]) {
                let _ = input.parse::<Token![in]>()?;
                let name = input.parse::<Ident>()?;
                if is_valid_tag(&name.to_string()) {
                    return Err(Error::new(
                        input.span(),
                        "Custom context cannot be an html element name",
                    ));
                } else {
                    input.parse::<Token![,]>().unwrap();
                    Some(name)
                }
            } else {
                None
            };

        let mut body = Vec::new();
        let mut children = Vec::new();
        let mut manual_props = None;
        let cfg: BodyParseConfig<AS_RSX> = BodyParseConfig {
            allow_children: true,
            allow_fields: false,
            allow_manual_props: false,
        };
        cfg.parse_component_body(input, &mut body, &mut children, &mut manual_props)?;

        Ok(Self {
            roots: children,
            custom_context,
        })
    }
}

/// The HTML variant of parsing rsx!
impl Parse for RsxBody<AS_HTML> {
    fn parse(input: ParseStream) -> Result<Self> {
        // if input.peek(LitStr) {
        //     return input.parse::<LitStr>()?.parse::<RsxRender>();
        // }

        // try to parse the first ident and comma
        let custom_context =
            if input.peek(Token![in]) && input.peek2(Ident) && input.peek3(Token![,]) {
                let _ = input.parse::<Token![in]>()?;
                let name = input.parse::<Ident>()?;
                if is_valid_tag(&name.to_string()) {
                    return Err(Error::new(
                        input.span(),
                        "Custom context cannot be an html element name",
                    ));
                } else {
                    input.parse::<Token![,]>().unwrap();
                    Some(name)
                }
            } else {
                None
            };

        let mut body = Vec::new();
        let mut children = Vec::new();
        let mut manual_props = None;

        let cfg: BodyParseConfig<AS_HTML> = BodyParseConfig {
            allow_children: true,
            allow_fields: false,
            allow_manual_props: false,
        };
        cfg.parse_component_body(input, &mut body, &mut children, &mut manual_props)?;

        Ok(Self {
            roots: children,
            custom_context,
        })
    }
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
                    use dioxus_elements::GlobalAttributes;

                    #inner
                }))
            }),
            // Otherwise we just build the LazyNode wrapper
            None => out_tokens.append_all(quote! {
                dioxus::prelude::LazyNodes::new(move |__cx: NodeFactory|{
                    use dioxus_elements::GlobalAttributes;

                    #inner
                 })
            }),
        };
    }
}
