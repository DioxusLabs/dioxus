#![allow(unused)]

#[cfg(feature = "hot_reload")]
use internment::Intern;

use proc_macro2::TokenStream as TokenStream2;
use std::{fmt::Debug, hash::Hash};
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseBuffer},
    Ident,
};

/// interns a object into a static object, resusing the value if it already exists
#[cfg(feature = "hot_reload")]
pub(crate) fn intern<T: Eq + Hash + Send + Sync + ?Sized + 'static>(
    s: impl Into<Intern<T>>,
) -> &'static T {
    s.into().as_ref()
}

/// These are just helpful methods for tests to pretty print the token stream - they are not used in the actual code
// #[cfg(test)]
pub trait PrettyUnparse {
    fn pretty_unparse(&self) -> String;
}

// #[cfg(test)]
impl PrettyUnparse for TokenStream2 {
    fn pretty_unparse(&self) -> String {
        let parsed = syn::parse2::<syn::Expr>(self.clone()).unwrap();
        prettier_please::unparse_expr(&parsed)
    }
}

/// Parse a raw ident and return a new ident with the r# prefix added
pub fn parse_raw_ident(parse_buffer: &ParseBuffer) -> syn::Result<Ident> {
    // First try to parse as a normal ident
    if let Ok(ident) = Ident::parse(parse_buffer) {
        return Ok(ident);
    }
    // If that fails, try to parse as a raw ident
    let ident = Ident::parse_any(parse_buffer)?;
    Ok(Ident::new_raw(&ident.to_string(), ident.span()))
}
