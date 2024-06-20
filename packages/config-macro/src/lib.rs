#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

macro_rules! define_config_macro {
    ($name:ident if $($cfg:tt)+) => {
        #[proc_macro]
        pub fn $name(input: TokenStream) -> TokenStream {
            if cfg!($($cfg)+) {
                let input = TokenStream2::from(input);
                quote! {
                    {
                        #input
                    }
                }
            } else {
                quote! {
                    {}
                }
            }
            .into()
        }
    };
}

define_config_macro!(server_only if any(feature = "ssr", feature = "liveview"));
define_config_macro!(client if any(feature = "desktop", feature = "web"));
define_config_macro!(web if feature = "web");
define_config_macro!(desktop if feature = "desktop");
define_config_macro!(mobile if feature = "mobile");
define_config_macro!(fullstack if feature = "fullstack");
define_config_macro!(static_generation if feature = "static-generation");
define_config_macro!(ssr if feature = "ssr");
define_config_macro!(liveview if feature = "liveview");
