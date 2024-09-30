use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned, ToTokens};
use serde::Serialize;
use std::sync::atomic::AtomicBool;
use syn::{parse::Parse, parse_macro_input, LitStr};

/// this new approach will store the assets descriptions *inside the executable*.
/// The trick is to use the `link_section` attribute.
/// We force rust to store a json representation of the asset description
/// inside a particular region of the binary, with the label "manganis".
/// After linking, the "manganis" sections of the different executables will be merged.
pub fn generate_link_section(asset: &impl Serialize) -> TokenStream2 {
    let position = proc_macro2::Span::call_site();

    let asset_description = serde_json::to_string(asset).unwrap();

    let len = asset_description.as_bytes().len();

    let asset_bytes = syn::LitByteStr::new(asset_description.as_bytes(), position);

    let section_name = syn::LitStr::new(LinkSection::CURRENT.link_section, position);

    quote! {
        #[link_section = #section_name]
        #[used]
        static ASSET: [u8; #len] = * #asset_bytes;
    }
}

/// Information about the manganis link section for a given platform
#[derive(Debug, Clone, Copy)]
struct LinkSection {
    /// The link section we pass to the static
    pub link_section: &'static str,
    /// The name of the section we find in the binary
    pub name: &'static str,
}

impl LinkSection {
    /// The list of link sections for all supported platforms
    pub const ALL: &'static [&'static LinkSection] =
        &[Self::WASM, Self::MACOS, Self::WINDOWS, Self::ILLUMOS];

    /// Returns the link section used in linux, android, fuchsia, psp, freebsd, and wasm32
    pub const WASM: &'static LinkSection = &LinkSection {
        link_section: "manganis",
        name: "manganis",
    };

    /// Returns the link section used in macOS, iOS, tvOS
    pub const MACOS: &'static LinkSection = &LinkSection {
        link_section: "__DATA,manganis,regular,no_dead_strip",
        name: "manganis",
    };

    /// Returns the link section used in windows
    pub const WINDOWS: &'static LinkSection = &LinkSection {
        link_section: "mg",
        name: "mg",
    };

    /// Returns the link section used in illumos
    pub const ILLUMOS: &'static LinkSection = &LinkSection {
        link_section: "set_manganis",
        name: "set_manganis",
    };

    /// The link section used on the current platform
    pub const CURRENT: &'static LinkSection = {
        #[cfg(any(
            target_os = "none",
            target_os = "linux",
            target_os = "android",
            target_os = "fuchsia",
            target_os = "psp",
            target_os = "freebsd",
            target_arch = "wasm32"
        ))]
        {
            Self::WASM
        }

        #[cfg(any(target_os = "macos", target_os = "ios", target_os = "tvos"))]
        {
            Self::MACOS
        }

        #[cfg(target_os = "windows")]
        {
            Self::WINDOWS
        }

        #[cfg(target_os = "illumos")]
        {
            Self::ILLUMOS
        }
    };
}
