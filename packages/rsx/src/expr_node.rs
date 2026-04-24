use crate::{DynIdx, PartialExpr};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::parse::Parse;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ExprNode {
    pub expr: PartialExpr,
    pub dyn_idx: DynIdx,
}

impl ExprNode {
    pub fn span(&self) -> proc_macro2::Span {
        self.expr.span()
    }
}

impl Parse for ExprNode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // // If it's a single-line expression, we want to parse it without braces, fixing some issues with rust 2024 lifetimes
        // use syn::braced;
        // let forked = input.fork();
        // if forked.peek(syn::token::Brace) {
        //     let content;
        //     let _brace = braced!(content in forked);
        //     let as_expr: Result<syn::Expr, syn::Error> = content.parse();
        //     if as_expr.is_ok() && content.is_empty() {
        //         let content;
        //         let _brace = braced!(content in input);
        //         return Ok(Self {
        //             expr: content.parse()?,
        //             dyn_idx: DynIdx::default(),
        //         });
        //     }
        // }

        Ok(Self {
            expr: input.parse()?,
            dyn_idx: DynIdx::default(),
        })
    }
}

impl ToTokens for ExprNode {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let expr = &self.expr;
        tokens.append_all(quote! {
            {
                #[allow(unused_braces)]
                let ___nodes = dioxus_core::IntoDynNode::into_dyn_node(#expr);
                ___nodes
            }
        })
    }
}

#[test]
fn no_commas() {
    use prettier_please::PrettyUnparse;
    let input = quote! {
        div {
            {label("Hello, world!")},
        }
    };

    let _expr: crate::BodyNode = syn::parse2(input).unwrap();
    println!("{}", _expr.to_token_stream().pretty_unparse());
}
