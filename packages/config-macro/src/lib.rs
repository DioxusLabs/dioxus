#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

#[proc_macro]
pub fn server_only(input: TokenStream) -> TokenStream {
    if cfg!(any(feature = "ssr", feature = "liveview")) {
        let input = TokenStream2::from(input);
        quote! {
            #input
        }
    } else {
        quote! {
            ()
        }
    }
    .into()
}

#[proc_macro]
pub fn client(input: TokenStream) -> TokenStream {
    if cfg!(any(feature = "desktop", feature = "web", feature = "tui")) {
        let input = TokenStream2::from(input);
        quote! {
            #input
        }
    } else {
        quote! {
            ()
        }
    }
    .into()
}

#[proc_macro]
pub fn web(input: TokenStream) -> TokenStream {
    if cfg!(feature = "web") {
        let input = TokenStream2::from(input);
        quote! {
            #input
        }
    } else {
        quote! {
            ()
        }
    }
    .into()
}

#[proc_macro]
pub fn desktop(input: TokenStream) -> TokenStream {
    if cfg!(feature = "desktop") {
        let input = TokenStream2::from(input);
        quote! {
            #input
        }
    } else {
        quote! {
            ()
        }
    }
    .into()
}

#[proc_macro]
pub fn fullstack(input: TokenStream) -> TokenStream {
    if cfg!(feature = "web") {
        let input = TokenStream2::from(input);
        quote! {
            #input
        }
    } else {
        quote! {
            ()
        }
    }
    .into()
}

#[proc_macro]
pub fn ssr(input: TokenStream) -> TokenStream {
    if cfg!(feature = "ssr") {
        let input = TokenStream2::from(input);
        quote! {
            #input
        }
    } else {
        quote! {
            ()
        }
    }
    .into()
}

#[proc_macro]
pub fn liveview(input: TokenStream) -> TokenStream {
    if cfg!(feature = "liveview") {
        let input = TokenStream2::from(input);
        quote! {
            #input
        }
    } else {
        quote! {
            ()
        }
    }
    .into()
}

#[proc_macro]
pub fn tui(input: TokenStream) -> TokenStream {
    if cfg!(feature = "tui") {
        let input = TokenStream2::from(input);
        quote! {
            #input
        }
    } else {
        quote! {
            ()
        }
    }
    .into()
}
