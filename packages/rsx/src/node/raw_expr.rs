use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use std::hash;
use syn::{parse::Parse, spanned::Spanned};

use crate::location::CallerLocation;

#[derive(Clone, Debug)]
pub struct RawExpr {
    pub expr: TokenStream2,
    pub dyn_idx: CallerLocation,
}

impl RawExpr {
    pub fn span(&self) -> proc_macro2::Span {
        self.expr.span()
    }
}

impl Parse for RawExpr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Pull the brace and then parse the innards as TokenStream2 - not expr
        let content;
        syn::braced!(content in input);

        Ok(Self {
            expr: content.parse()?,
            dyn_idx: CallerLocation::default(),
        })
    }
}

impl ToTokens for RawExpr {
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

impl PartialEq for RawExpr {
    fn eq(&self, other: &Self) -> bool {
        self.expr.to_string() == other.expr.to_string()
    }
}

impl Eq for RawExpr {}

impl hash::Hash for RawExpr {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.expr.to_string().hash(state);
    }
}
