use crate::location::DynIdx;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use std::hash;
use syn::{parse::Parse, spanned::Spanned, token::Brace};

/// A raw expression wrapped in curly braces that is parsed from the input stream.
#[derive(Clone, Debug)]
pub struct PartialExpr {
    // todo: rstml uses the syn `Block` type which is more flexible on the receiving end than our
    // partially-complete TokenStream approach
    pub brace: Brace,
    pub expr: TokenStream2,
    pub dyn_idx: DynIdx,
}

impl Parse for PartialExpr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Pull the brace and then parse the innards as TokenStream2 - not expr
        let content;
        let brace = syn::braced!(content in input);

        Ok(Self {
            brace,
            expr: content.parse()?,
            dyn_idx: DynIdx::default(),
        })
    }
}

impl ToTokens for PartialExpr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let exp = &self.expr;

        // Make sure we bind the expression to a variable so the lifetimes are relaxed
        tokens.append_all(quote! {
            {
                let ___nodes = (#exp).into_dyn_node();
                ___nodes
            }
        })
    }
}

impl PartialExpr {
    pub fn span(&self) -> proc_macro2::Span {
        self.brace.span.span()
    }
}

impl PartialEq for PartialExpr {
    fn eq(&self, other: &Self) -> bool {
        self.expr.to_string() == other.expr.to_string()
    }
}

impl Eq for PartialExpr {}

impl hash::Hash for PartialExpr {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.expr.to_string().hash(state);
    }
}
