#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

fn group(delimiter: Delimiter, stream: TokenStream) -> TokenStream {
    TokenTree::Group(Group::new(delimiter, stream)).into()
}

macro_rules! define_config_macro {
    ($name:ident if $($cfg:tt)+) => {
        #[proc_macro]
        pub fn $name(input: TokenStream) -> TokenStream {
            if cfg!($($cfg)+) {
                group(Delimiter::Brace, input)
            } else {
                group(Delimiter::Parenthesis, TokenStream::new())
            }
        }
    };
}

define_config_macro!(server_only if any(feature = "ssr", feature = "liveview"));
define_config_macro!(client if any(feature = "desktop", feature = "web", feature = "mobile"));
define_config_macro!(web if feature = "web");
define_config_macro!(desktop if feature = "desktop");
define_config_macro!(native if feature = "native");
define_config_macro!(mobile if feature = "mobile");
define_config_macro!(fullstack if feature = "fullstack");
define_config_macro!(ssr if feature = "ssr");
define_config_macro!(liveview if feature = "liveview");
