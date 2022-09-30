use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, TokenStreamExt};
use syn::{parse::Parse, spanned::Spanned, Expr};

use crate::whitespace::Whitespace;

#[derive(Debug, PartialEq, Eq)]
pub struct RawExprNode {
    pub ws: Whitespace,
    pub expr: Expr,
}

impl RawExprNode {
    pub fn span(&self) -> Span {
        self.expr.span()
    }
}

impl Parse for RawExprNode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let before_cursor = input.cursor();

        let expr = input.parse()?;

        let after = input.cursor();

        dbg!(before_cursor.span().start(), after.span().end());

        Ok(Self {
            ws: Whitespace::default(),
            expr,
        })
    }
}

impl ToTokens for RawExprNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let expr = &self.expr;
        tokens.append_all(quote::quote! { #expr });
    }
}
