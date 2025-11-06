use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};

/// We store description of the assets an application uses in the executable.
/// We use the `link_section` attribute embed an extra section in the executable.
/// We force rust to store a serialized representation of the asset description
/// inside a particular region of the binary, with the label "manganis".
/// After linking, the "manganis" sections of the different object files will be merged.
pub fn generate_link_section(asset: impl ToTokens, asset_hash: &str) -> TokenStream2 {
    dx_macro_helpers::linker::generate_link_section(
        asset,
        asset_hash,
        "__MANGANIS__",
        quote! { manganis::macro_helpers::serialize_asset },
        quote! { manganis::macro_helpers::copy_bytes },
        quote! { manganis::macro_helpers::const_serialize::ConstVec<u8> },
        true, // Add #[used] attribute for defense-in-depth, even though we also reference it
    )
}
