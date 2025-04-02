use std::{
    cell::OnceCell,
    hash::{Hash, Hasher},
};

use quote::{quote, ToTokens};
use syn::Ident;

use crate::PartialExpr;

#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub(crate) struct ExpressionPool {
    pub(crate) expressions: Vec<PartialExpr>,
}

impl ToTokens for ExpressionPool {
    fn to_tokens(&self, out: &mut proc_macro2::TokenStream) {
        let assignments = self.expressions.iter().enumerate().map(|(idx, expr)| {
            let ident = ident_for_index(idx, expr);
            quote! {
                let #ident = #expr;
            }
        });
        quote! {#(#assignments)*}.to_tokens(out);
    }
}

impl ExpressionPool {
    fn ident_for_index(&self, idx: usize) -> syn::Ident {
        ident_for_index(idx, &self.expressions[idx])
    }

    pub(crate) fn add(&mut self, expr: PartialExpr) -> Ident {
        let idx = self.expressions.len();
        self.expressions.push(expr.clone());
        self.ident_for_index(idx)
    }
}

fn ident_for_index(idx: usize, expression: &PartialExpr) -> Ident {
    let ident = format!("__temp_{}", idx);
    syn::Ident::new(&ident, expression.span())
}

#[derive(PartialEq, Eq, Clone, Debug, Default)]
/// An expression location in the expression pool.
pub(crate) struct OutOfOrderExpression {
    ident: OnceCell<Ident>,
}

impl ToTokens for OutOfOrderExpression {
    fn to_tokens(&self, out: &mut proc_macro2::TokenStream) {
        if let Some(ident) = self.get() {
            ident.to_tokens(out);
        } else {
            panic!("OutOfOrderExpression not initialized");
        }
    }
}

impl Hash for OutOfOrderExpression {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ident.get().hash(state);
    }
}

impl OutOfOrderExpression {
    pub(crate) fn insert(&self, ident: Ident) {
        self.ident.set(ident).unwrap();
    }

    pub(crate) fn get(&self) -> Option<&Ident> {
        self.ident.get()
    }
}
