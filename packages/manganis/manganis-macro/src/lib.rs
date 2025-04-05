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
/// # use manganis::{asset, Asset};
/// const _: Asset = asset!("/assets/asset.txt");
/// ```
/// Macros like `concat!` and `env!` are supported in the asset path.
/// ```rust
/// # use manganis::{asset, Asset};
/// const _: Asset = asset!(concat!("/assets/", env!("CARGO_CRATE_NAME"), ".dat"));
/// ```
///
/// # Images
///
/// You can collect images which will be automatically optimized with the image builder:
/// ```rust
/// # use manganis::{asset, Asset};
/// const _: Asset = asset!("/assets/image.png");
/// ```
/// Resize the image at compile time to make the assets file size smaller:
/// ```rust
/// # use manganis::{asset, Asset, ImageAssetOptions, ImageSize};
/// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_size(ImageSize::Manual { width: 52, height: 52 }));
/// ```
/// Or convert the image at compile time to a web friendly format:
/// ```rust
/// # use manganis::{asset, Asset, ImageAssetOptions, ImageSize, ImageFormat};
/// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_format(ImageFormat::Avif));
/// ```
/// You can mark images as preloaded to make them load faster in your app
/// ```rust
/// # use manganis::{asset, Asset, ImageAssetOptions};
/// const _: Asset = asset!("/assets/image.png", ImageAssetOptions::new().with_preload(true));
/// ```
#[proc_macro]
pub fn asset(input: TokenStream) -> TokenStream {
    let asset = parse_macro_input!(input as asset::AssetParser);

    quote! { #asset }.into_token_stream().into()
}
