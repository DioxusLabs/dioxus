use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};

/// Generate a linker section for embedding permission data in the binary
///
/// This function creates a static array containing the serialized permission data
/// wrapped in SymbolData::Permission and exports it with the __ASSETS__ prefix
/// for unified symbol collection with assets.
pub fn generate_link_section(permission: impl ToTokens, permission_hash: &str) -> TokenStream2 {
    dx_macro_helpers::linker::generate_link_section(
        permission,
        permission_hash,
        "__ASSETS__",
        quote! { permissions::macro_helpers::serialize_permission },
        quote! { permissions::macro_helpers::copy_bytes },
        quote! { permissions::macro_helpers::ConstVec<u8> },
        true, // permissions needs #[used] attribute
    )
}
