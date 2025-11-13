use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};

/// Generate a linker section for embedding asset data in the binary
///
/// This function creates a static array containing the serialized asset data
/// and exports it with the __ASSETS__ prefix for unified symbol collection.
/// Uses the generic linker helper from dx-macro-helpers for consistency.
pub fn generate_link_section(asset: impl ToTokens, asset_hash: &str) -> TokenStream2 {
    dx_macro_helpers::linker::generate_link_section(
        asset,
        asset_hash,
        "__ASSETS__",
        quote! { manganis::macro_helpers::serialize_asset },
        quote! { manganis::macro_helpers::copy_bytes },
        quote! { manganis::macro_helpers::ConstVec<u8, 4096> },
        false, // assets don't need #[used] attribute
    )
}
