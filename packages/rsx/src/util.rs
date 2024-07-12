#![allow(unused)]

#[cfg(feature = "hot_reload")]
use internment::Intern;

use proc_macro2::TokenStream as TokenStream2;
use std::{fmt::Debug, hash::Hash};

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
