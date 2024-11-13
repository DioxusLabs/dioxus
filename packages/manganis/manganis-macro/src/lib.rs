#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::parse_macro_input;

pub(crate) mod asset;
pub(crate) mod linker;

use linker::generate_link_section;

/// The asset macro collects assets that will be included in the final binary
#[proc_macro]
pub fn asset(input: TokenStream) -> TokenStream {
    let asset = parse_macro_input!(input as asset::AssetParser);

    quote! { #asset }.into_token_stream().into()
}
