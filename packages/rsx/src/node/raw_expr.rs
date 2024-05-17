use proc_macro2::TokenStream as TokenStream2;
use std::hash;

#[derive(Clone, Debug)]
pub struct RawExpr {
    pub expr: TokenStream2,
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
