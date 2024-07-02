use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::Parse;

use crate::{DynIdx, PartialExpr};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ExprNode {
    pub expr: PartialExpr,
    pub dyn_idx: DynIdx,
}

impl Parse for ExprNode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
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
                let ___nodes = ({
                    #expr
                }).into_dyn_node();

                ___nodes
            }
        })
    }
}
