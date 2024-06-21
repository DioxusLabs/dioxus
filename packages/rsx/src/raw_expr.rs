use crate::location::DynIdx;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use std::hash;
use syn::{parse::Parse, spanned::Spanned, token::Brace};

/// A raw expression wrapped in curly braces that is parsed from the input stream.
#[derive(Clone, Debug)]
pub struct BracedRawExpr {
    // todo: rstml uses the syn `Block` type which is more flexible on the receiving end than our
    // partially-complete TokenStream approach
    pub brace: Option<Brace>,
    pub expr: TokenStream2,
    pub dyn_idx: DynIdx,
}

impl BracedRawExpr {
    pub fn span(&self) -> proc_macro2::Span {
        self.expr.span()
    }
}

impl Parse for BracedRawExpr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Pull the brace and then parse the innards as TokenStream2 - not expr
        let content;
        let brace = syn::braced!(content in input);

        Ok(Self {
            expr: content.parse()?,
            brace: Some(brace),
            dyn_idx: DynIdx::default(),
        })
    }
}

impl ToTokens for BracedRawExpr {
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

impl PartialEq for BracedRawExpr {
    fn eq(&self, other: &Self) -> bool {
        self.expr.to_string() == other.expr.to_string()
    }
}

impl Eq for BracedRawExpr {}

impl hash::Hash for BracedRawExpr {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.expr.to_string().hash(state);
    }
}
