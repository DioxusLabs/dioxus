#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

// use css::CssAssetParser;
// use file::FileAssetParser;
// use folder::FolderAssetParser;
// use font::FontAssetParser;
// use image::ImageAssetParser;
// use js::JsAssetParser;
// use json::JsonAssetParser;
// use manganis_common::cache::macro_log_file;
use manganis_common::{MetadataAsset, ResourceAsset, TailwindAsset};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned, ToTokens};
// use resource::ResourceAssetParser;
use serde::Serialize;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use syn::{parse::Parse, parse_macro_input, LitStr};

pub(crate) mod asset;
// pub(crate) mod css;
// pub(crate) mod file;
// pub(crate) mod folder;
// pub(crate) mod font;
// pub(crate) mod image;
// pub(crate) mod js;
// pub(crate) mod json;
// pub(crate) mod resource;

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

    let section_name = syn::LitStr::new(
        manganis_common::linker::LinkSection::CURRENT.link_section,
        position,
    );

    quote! {
        #[link_section = #section_name]
        #[used]
        static ASSET: [u8; #len] = * #asset_bytes;
    }
}

/// Collects tailwind classes that will be included in the final binary and returns them unmodified
///
/// ```rust
/// // You can include tailwind classes that will be collected into the final binary
/// const TAILWIND_CLASSES: &str = manganis::classes!("flex flex-col p-5");
/// assert_eq!(TAILWIND_CLASSES, "flex flex-col p-5");
/// ```
#[proc_macro]
pub fn classes(input: TokenStream) -> TokenStream {
    trace_to_file();

    let input_as_str = parse_macro_input!(input as LitStr);
    let input_as_str = input_as_str.value();

    let asset = TailwindAsset::new(&input_as_str);
    let link_section = generate_link_section(&asset);

    quote! {
        {
            #link_section
            #input_as_str
        }
    }
    .into_token_stream()
    .into()
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

    let md = parse_macro_input!(input as MetadataValue);
    let asset = MetadataAsset::new(md.key.as_str(), md.value.as_str());
    let link_section = generate_link_section(&asset);

    quote! {
        {
            #link_section
        }
    }
    .into_token_stream()
    .into()
}

// #[cfg(feature = "url-encoding")]
// pub(crate) fn url_encoded_asset(
//     file_asset: &manganis_common::ResourceAsset,
// ) -> Result<String, syn::Error> {
//     use base64::Engine;

//     let target_directory =
//         std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
//     let output_folder = std::path::Path::new(&target_directory)
//         .join("manganis")
//         .join("assets");
//     std::fs::create_dir_all(&output_folder).map_err(|e| {
//         syn::Error::new(
//             proc_macro2::Span::call_site(),
//             format!("Failed to create output folder: {}", e),
//         )
//     })?;
//     manganis_cli_support::process_file(file_asset, &output_folder).map_err(|e| {
//         syn::Error::new(
//             proc_macro2::Span::call_site(),
//             format!("Failed to process file: {}", e),
//         )
//     })?;
//     let file = output_folder.join(file_asset.location().unique_name());
//     let data = std::fs::read(file).map_err(|e| {
//         syn::Error::new(
//             proc_macro2::Span::call_site(),
//             format!("Failed to read file: {}", e),
//         )
//     })?;
//     let data = base64::engine::general_purpose::STANDARD_NO_PAD.encode(data);
//     let mime = manganis_common::get_mime_from_ext(file_asset.options().extension());
//     Ok(format!("data:{mime};base64,{data}"))
// }

pub(crate) fn verify_preload_valid(ident: &Ident) -> Result<(), syn::Error> {
    // Compile time preload is only supported for the primary package
    if std::env::var("CARGO_PRIMARY_PACKAGE").is_err() {
        return Err(syn::Error::new(
            ident.span(),
            "The `preload` option is only supported for the primary package. Libraries should not preload assets or should preload assets\
            at runtime with utilities your framework provides",
        ));
    }

    Ok(())
}
