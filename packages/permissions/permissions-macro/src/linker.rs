use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};

/// Generate a linker section for embedding permission data in the binary
///
/// This function creates a static array containing the serialized permission data
/// and exports it with a unique symbol name that can be found by build tools.
/// Uses the unified __MANGANIS__ prefix to share the same symbol collection as assets.
pub fn generate_link_section(permission: impl ToTokens, permission_hash: &str) -> TokenStream2 {
    dx_macro_helpers::linker::generate_link_section(
        permission,
        permission_hash,
        "__MANGANIS__",
        quote! { permissions::macro_helpers::serialize_linker_symbol_permission },
        quote! { permissions::macro_helpers::copy_bytes },
        quote! { permissions::macro_helpers::ConstVec<u8, 4096> },
        false, // No #[used] attribute - we use volatile reads instead
    )
}
