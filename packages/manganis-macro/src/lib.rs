#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::parse_macro_input;

pub(crate) mod asset;
pub(crate) mod linker;

use linker::generate_link_section;

/// The asset macro collects assets that will be included in the final binary
///
/// # Files
///
/// The file builder collects an arbitrary file. Relative paths are resolved relative to the package root
/// ```rust
/// # use manganis::asset;
/// const _: &str = asset!("src/asset.txt");
/// ```
/// Or you can use URLs to read the asset at build time from a remote location
/// ```rust
/// # use manganis::asset;
/// const _: &str = asset!("https://rustacean.net/assets/rustacean-flat-happy.png");
/// ```
///
/// # Images
///
/// You can collect images which will be automatically optimized with the image builder:
/// ```rust
/// # use manganis::asset;
/// const _: manganis::ImageAsset = asset!(image("rustacean-flat-gesture.png"));
/// ```
/// Resize the image at compile time to make the assets file size smaller:
/// ```rust
/// # use manganis::asset;
/// const _: manganis::ImageAsset = asset!(image("rustacean-flat-gesture.png").size(52, 52));
/// ```
/// Or convert the image at compile time to a web friendly format:
/// ```rust
/// # use manganis::asset;
/// const _: manganis::ImageAsset = asset!(image("rustacean-flat-gesture.png").format(ImageFormat::Avif).size(52, 52));
/// ```
/// You can mark images as preloaded to make them load faster in your app
/// ```rust
/// # use manganis::asset;
/// const _: manganis::ImageAsset = asset!(image("rustacean-flat-gesture.png").preload());
/// ```
#[proc_macro]
pub fn asset(input: TokenStream) -> TokenStream {
    let asset = parse_macro_input!(input as asset::AssetParser);

    quote! { #asset }.into_token_stream().into()
}
