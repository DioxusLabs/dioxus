#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn server_only(input: TokenStream) -> TokenStream {
    if cfg!(any(feature = "ssr", feature = "liveview")) {
        input
    } else {
        quote! {
            {}
        }
        .into()
    }
}

#[proc_macro]
pub fn client(input: TokenStream) -> TokenStream {
    if cfg!(any(feature = "desktop", feature = "web")) {
        input
    } else {
        quote! {
            {}
        }
        .into()
    }
}

#[proc_macro]
pub fn web(input: TokenStream) -> TokenStream {
    if cfg!(feature = "web") {
        input
    } else {
        quote! {
            {}
        }
        .into()
    }
}

#[proc_macro]
pub fn desktop(input: TokenStream) -> TokenStream {
    if cfg!(feature = "desktop") {
        input
    } else {
        quote! {
            {}
        }
        .into()
    }
}

#[proc_macro]
pub fn mobile(input: TokenStream) -> TokenStream {
    if cfg!(feature = "mobile") {
        input
    } else {
        quote! {
            {}
        }
        .into()
    }
}

#[proc_macro]
pub fn fullstack(input: TokenStream) -> TokenStream {
    if cfg!(feature = "fullstack") {
        input
    } else {
        quote! {
            {}
        }
        .into()
    }
}

#[proc_macro]
pub fn static_generation(input: TokenStream) -> TokenStream {
    if cfg!(feature = "static-generation") {
        input
    } else {
        quote! {
            {}
        }
        .into()
    }
}

#[proc_macro]
pub fn ssr(input: TokenStream) -> TokenStream {
    if cfg!(feature = "ssr") {
        input
    } else {
        quote! {
            {}
        }
        .into()
    }
}

#[proc_macro]
pub fn liveview(input: TokenStream) -> TokenStream {
    if cfg!(feature = "liveview") {
        input
    } else {
        quote! {
            {}
        }
        .into()
    }
}
