#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

use component::ComponentBody;
use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

mod component;
mod props;
mod utils;

use dioxus_rsx as rsx;

#[doc = include_str!("../docs/rsx.md")]
#[proc_macro]
pub fn rsx(tokens: TokenStream) -> TokenStream {
    match syn::parse::<rsx::CallBody>(tokens) {
        Err(err) => err.to_compile_error().into(),
        Ok(body) => body.into_token_stream().into(),
    }
}

#[doc = include_str!("../docs/component.md")]
#[proc_macro_attribute]
pub fn component(_args: TokenStream, input: TokenStream) -> TokenStream {
    parse_macro_input!(input as ComponentBody)
        .into_token_stream()
        .into()
}

#[doc = include_str!("../docs/props.md")]
#[proc_macro_derive(Props, attributes(props))]
pub fn derive_props(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match props::impl_my_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
