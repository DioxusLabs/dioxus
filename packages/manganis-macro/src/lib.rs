#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned, ToTokens};
use serde::Serialize;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use syn::{parse::Parse, parse_macro_input, LitStr};
pub(crate) mod asset;

static LOG_FILE_FRESH: AtomicBool = AtomicBool::new(false);

fn trace_to_file() {
    // // If this is the first time the macro is used in the crate, set the subscriber to write to a file
    // if !LOG_FILE_FRESH.fetch_or(true, Ordering::Relaxed) {
    //     let path = macro_log_file();
    //     std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    //     let file = std::fs::OpenOptions::new()
    //         .create(true)
    //         .write(true)
    //         .truncate(true)
    //         .open(path)
    //         .unwrap();
    //     tracing_subscriber::fmt::fmt().with_writer(file).init();
    // }
}

/// this new approach will store the assets descriptions *inside the executable*.
/// The trick is to use the `link_section` attribute.
/// We force rust to store a json representation of the asset description
/// inside a particular region of the binary, with the label "manganis".
/// After linking, the "manganis" sections of the different executables will be merged.
fn generate_link_section(asset: &impl Serialize) -> TokenStream2 {
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

/// The mg macro collects assets that will be included in the final binary
///
/// # Files
///
/// The file builder collects an arbitrary file. Relative paths are resolved relative to the package root
/// ```rust
/// const _: &str = manganis::asset!("src/asset.txt");
/// ```
/// Or you can use URLs to read the asset at build time from a remote location
/// ```rust
/// const _: &str = manganis::asset!("https://rustacean.net/assets/rustacean-flat-happy.png");
/// ```
///
/// # Images
///
/// You can collect images which will be automatically optimized with the image builder:
/// ```rust
/// const _: manganis::ImageAsset = manganis::asset!(image("rustacean-flat-gesture.png"));
/// ```
/// Resize the image at compile time to make the assets file size smaller:
/// ```rust
/// const _: manganis::ImageAsset = manganis::asset!(image("rustacean-flat-gesture.png").size(52, 52));
/// ```
/// Or convert the image at compile time to a web friendly format:
/// ```rust
/// const _: manganis::ImageAsset = manganis::asset!(image("rustacean-flat-gesture.png").format(ImageFormat::Avif).size(52, 52));
/// ```
/// You can mark images as preloaded to make them load faster in your app
/// ```rust
/// const _: manganis::ImageAsset = manganis::asset!(image("rustacean-flat-gesture.png").preload());
/// ```
///
/// # Fonts
///
/// You can use the font builder to collect fonts that will be included in the final binary from google fonts
/// ```rust
/// const _: &str = manganis::asset!(font().families(["Roboto"]));
/// ```
/// You can specify weights for the fonts
/// ```rust
/// const _: &str = manganis::asset!(font().families(["Roboto"]).weights([200]));
/// ```
/// Or set the text to only include the characters you need
/// ```rust
/// const _: &str = manganis::asset!(font().families(["Roboto"]).weights([200]).text("Hello, world!"));
/// ```
#[proc_macro]
pub fn asset(input: TokenStream) -> TokenStream {
    trace_to_file();
    let asset = parse_macro_input!(input as asset::AssetParser);

    quote! {
        #asset
    }
    .into_token_stream()
    .into()
}

/// // You can also collect arbitrary key-value pairs. The meaning of these pairs is determined by the CLI that processes your assets
/// ```rust
/// const _: () = manganis::meta!("opt-level": "3");
/// ```
#[proc_macro]
pub fn meta(input: TokenStream) -> TokenStream {
    struct MetadataValue {
        key: String,
        value: String,
    }

    impl Parse for MetadataValue {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let key = input.parse::<Ident>()?.to_string();
            input.parse::<syn::Token![:]>()?;
            let value = input.parse::<LitStr>()?.value();
            Ok(Self { key, value })
        }
    }

    trace_to_file();

    todo!()

    // let md = parse_macro_input!(input as MetadataValue);
    // let asset = MetadataAsset::new(md.key.as_str(), md.value.as_str());
    // let link_section = generate_link_section(&asset);

    // quote! {
    //     {
    //         #link_section
    //     }
    // }
    // .into_token_stream()
    // .into()
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
