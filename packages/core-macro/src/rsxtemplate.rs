use crate::{rsxt::RsxRender, util::is_valid_svg_tag};

use {
    proc_macro::TokenStream,
    proc_macro2::{Span, TokenStream as TokenStream2},
    quote::{quote, ToTokens, TokenStreamExt},
    syn::{
        ext::IdentExt,
        parse::{Parse, ParseStream},
        token, Error, Expr, ExprClosure, Ident, LitBool, LitStr, Path, Result, Token,
    },
};

// ==============================================
// Parse any stream coming from the html! macro
// ==============================================
pub struct RsxTemplate {
    inner: RsxRender,
}

impl Parse for RsxTemplate {
    fn parse(s: ParseStream) -> Result<Self> {
        if s.peek(LitStr) {
            use std::str::FromStr;

            let lit = s.parse::<LitStr>()?;
            let g = lit.span();
            let mut value = lit.value();
            if value.ends_with('\n') {
                value.pop();
                if value.ends_with('\r') {
                    value.pop();
                }
            }
            let lit = LitStr::new(&value, lit.span());

            // panic!("{:#?}", lit);
            match lit.parse::<crate::rsxt::RsxRender>() {
                Ok(r) => Ok(Self { inner: r }),
                Err(e) => Err(e),
            }
        } else {
            panic!("Not a str lit")
        }
        // let t = s.parse::<LitStr>()?;

        // let new_stream = TokenStream::from(t.to_s)

        // let ctx: Ident = s.parse()?;
        // s.parse::<Token![,]>()?;
        // if elements are in an array, return a bumpalo::collections::Vec rather than a Node.
        // let kind = if s.peek(token::Bracket) {
        //     let nodes_toks;
        //     syn::bracketed!(nodes_toks in s);
        //     let mut nodes: Vec<MaybeExpr<Node>> = vec![nodes_toks.parse()?];
        //     while nodes_toks.peek(Token![,]) {
        //         nodes_toks.parse::<Token![,]>()?;
        //         nodes.push(nodes_toks.parse()?);
        //     }
        //     NodeOrList::List(NodeList(nodes))
        // } else {
        //     NodeOrList::Node(s.parse()?)
        // };
        // Ok(HtmlRender { kind })
    }
}

impl ToTokens for RsxTemplate {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        self.inner.to_tokens(out_tokens);
        // let new_toks = ToToksCtx::new(&self.kind).to_token_stream();

        // // create a lazy tree that accepts a bump allocator
        // let final_tokens = quote! {
        //     dioxus::prelude::LazyNodes::new(move |ctx| {
        //         let bump = &ctx.bump();

        //         #new_toks
        //     })
        // };

        // final_tokens.to_tokens(out_tokens);
    }
}
