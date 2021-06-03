//! Parse `Fragments` into the Fragment VNode
//! ==========================================
//!
//! This parsing path emerges from [`AmbiguousElement`] which supports validation of the Fragment format.
//! We can be reasonably sure that whatever enters this parsing path is in the right format.
//! This feature must support:
//! - [x] Optional commas
//! - [ ] Children
//! - [ ] Keys

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

pub struct Fragment {}
impl Parse for Fragment {
    fn parse(input: ParseStream) -> Result<Self> {
        todo!()
    }
}

impl ToTokens for Fragment {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        todo!()
    }
}
