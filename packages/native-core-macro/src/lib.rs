extern crate proc_macro;

mod sorted_slice;

use proc_macro::TokenStream;
use quote::{quote, ToTokens, __private::Span};
use sorted_slice::StrSlice;
use syn::parenthesized;
use syn::parse::ParseBuffer;
use syn::punctuated::Punctuated;
use syn::{
    self,
    parse::{Parse, ParseStream, Result},
    parse_macro_input, parse_quote, Error, Field, Ident, Token, Type,
};

/// A helper attribute for deriving `State` for a struct.
#[proc_macro_attribute]
pub fn my_attribute(_: TokenStream, input: TokenStream) -> TokenStream {
    let impl_block: syn::ItemImpl = syn::parse(input).unwrap();
    quote!(#impl_block).into()
}
