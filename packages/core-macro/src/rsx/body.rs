use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Ident, Result, Token,
};

use super::*;

pub struct CallBody {
    custom_context: Option<Ident>,
    roots: Vec<BodyNode>,
}

/// The custom rusty variant of parsing rsx!
impl Parse for CallBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let custom_context = try_parse_custom_context(input)?;
        let (_, roots, _) = BodyConfig::new_call_body().parse_component_body(input)?;
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
impl ToTokens for CallBody {
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
                #ident.render(dioxus::prelude::LazyNodes::new(move |__cx: NodeFactory| -> VNode {
                    use dioxus_elements::{GlobalAttributes, SvgAttributes};

                    #inner
                }))
            }),
            // Otherwise we just build the LazyNode wrapper
            None => out_tokens.append_all(quote! {
                {
                    NodeFactory::annotate_lazy(move |__cx: NodeFactory| -> VNode {
                        use dioxus_elements::{GlobalAttributes, SvgAttributes};
                        #inner
                    })
                }
            }),
        };
    }
}
