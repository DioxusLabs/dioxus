use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;

/// Generate a linker section for embedding permission data in the binary
///
/// This function creates a static array containing the serialized permission data
/// and exports it with a unique symbol name that can be found by build tools.
/// The pattern follows the same approach as Manganis for asset embedding.
pub fn generate_link_section(permission: impl ToTokens, permission_hash: &str) -> TokenStream2 {
    let position = proc_macro2::Span::call_site();
    let export_name = syn::LitStr::new(&format!("__PERMISSION__{}", permission_hash), position);

    quote::quote! {
        // First serialize the permission into a constant sized buffer
        const __BUFFER: permissions::macro_helpers::ConstVec<u8, 4096> =
            permissions::macro_helpers::serialize_permission(&#permission);
        // Then pull out the byte slice
        const __BYTES: &[u8] = __BUFFER.as_ref();
        // And the length of the byte slice
        const __LEN: usize = __BYTES.len();

        // Now that we have the size of the permission, copy the bytes into a static array
        #[used]
        #[unsafe(export_name = #export_name)]
        static __LINK_SECTION: [u8; __LEN] = permissions::macro_helpers::copy_bytes(__BYTES);
    }
}
