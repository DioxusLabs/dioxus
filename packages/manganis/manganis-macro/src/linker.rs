use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::quote;

/// Generate a linker section for embedding arbitrary data in the binary
///
/// This is a generic version that allows customizing the serialization function,
/// buffer type, and copy bytes function. Used by both asset and FFI metadata embedding.
pub fn generate_link_section_inner(
    item: TokenStream2,
    hash: &str,
    prefix: &str,
    serialize_fn: TokenStream2,
    copy_bytes_fn: TokenStream2,
    buffer_type: TokenStream2,
) -> TokenStream2 {
    let position = proc_macro2::Span::call_site();
    let export_name = syn::LitStr::new(&format!("{}{}", prefix, hash), position);

    quote! {
        #[used]
        static __LINK_SECTION: &'static [u8] = {
            const __BUFFER: #buffer_type = #serialize_fn(&#item);
            const __BYTES: &[u8] = __BUFFER.as_ref();
            const __LEN: usize = __BYTES.len();

            #[unsafe(export_name = #export_name)]
            #[used]
            static __LINK_SECTION: [u8; __LEN] = #copy_bytes_fn(__BYTES);
            &__LINK_SECTION
        };
    }
}

/// Generate a linker section for embedding asset data in the binary
///
/// This function creates a static array containing the serialized asset data
/// and exports it with the __ASSETS__ prefix for unified symbol collection.
pub fn generate_link_section(asset: impl ToTokens, asset_hash: &str) -> TokenStream2 {
    let item = asset;
    let serialize_fn = quote! { manganis::macro_helpers::serialize_asset };
    let copy_bytes_fn = quote! { manganis::macro_helpers::copy_bytes };
    let buffer_type = quote! { manganis::macro_helpers::ConstVec<u8, 4096> };
    let position = proc_macro2::Span::call_site();
    let export_name = syn::LitStr::new(&format!("__ASSETS__{}", asset_hash), position);

    quote! {
        static __LINK_SECTION: &'static [u8] = {
            const __BUFFER: #buffer_type = #serialize_fn(&#item);
            const __BYTES: &[u8] = __BUFFER.as_ref();
            const __LEN: usize = __BYTES.len();

            #[unsafe(export_name = #export_name)]
            static __LINK_SECTION: [u8; __LEN] = #copy_bytes_fn(__BYTES);
            &__LINK_SECTION
        };
    }
}
