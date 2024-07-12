use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use std::hash;
use syn::{parse::Parse, token::Brace, Expr};

/// A raw expression potentially wrapped in curly braces that is parsed from the input stream.
///
/// If there are no braces, it tries to parse as an expression without partial expansion. If there
/// are braces, it parses the contents as a `TokenStream2` and stores it as such.
#[derive(Clone, Debug)]
pub struct PartialExpr {
    pub brace: Option<Brace>,
    pub expr: TokenStream2,
}

impl Parse for PartialExpr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse as an expression if there is no brace
        if !input.peek(syn::token::Brace) {
            let expr = input.parse::<syn::Expr>()?;
            return Ok(Self {
                brace: None,
                expr: quote! { #expr },
            });
        }

        let content;
        let brace = Some(syn::braced!(content in input));
        let expr = content.parse()?;

        Ok(Self { brace, expr })
    }
}

impl ToTokens for PartialExpr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match &self.brace {
            Some(brace) => brace.surround(tokens, |tokens| self.expr.to_tokens(tokens)),
            _ => self.expr.to_tokens(tokens),
        }
    }
}

impl PartialExpr {
    pub fn as_expr(&self) -> syn::Result<syn::Expr> {
        syn::parse2(self.expr.clone())
    }

    pub fn from_expr(expr: &Expr) -> Self {
        Self {
            brace: None,
            expr: quote! { #expr },
        }
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
