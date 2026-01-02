use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use quote::ToTokens;

/// Generate a linker section for embedding asset data in the binary
///
/// This function creates a static array containing the serialized asset data
/// and exports it with the __ASSETS__ prefix for unified symbol collection.
/// Uses the generic linker helper from dx-macro-helpers for consistency.
pub fn generate_link_section(asset: impl ToTokens, asset_hash: &str) -> TokenStream2 {
    generate_link_sections_with_legacy(
        asset,
        asset_hash,
        "__ASSETS__",
        "__MANGANIS__",
        quote! { manganis::macro_helpers::serialize_asset },
        quote! { manganis::macro_helpers::serialize_asset_07 },
        quote! { manganis::macro_helpers::copy_bytes },
        quote! { manganis::macro_helpers::ConstVec<u8, 4096> },
        quote! { manganis::macro_helpers::const_serialize_07::ConstVec<u8> },
        false,
    )
}

/// Generate a pair of linker sections for legacy + current formats
///
/// This is useful when emitting both old and new symbol formats for compatibility.
#[allow(clippy::too_many_arguments)]
pub fn generate_link_sections_with_legacy(
    item: impl ToTokens,
    hash: &str,
    prefix_current: &str,
    prefix_legacy: &str,
    serialize_fn_current: TokenStream2,
    serialize_fn_legacy: TokenStream2,
    copy_bytes_fn: TokenStream2,
    buffer_type_current: TokenStream2,
    buffer_type_legacy: TokenStream2,
    add_used_attribute: bool,
) -> TokenStream2 {
    let position = proc_macro2::Span::call_site();
    let export_name = syn::LitStr::new(&format!("{}{}", prefix_current, hash), position);
    let legacy_export_name = syn::LitStr::new(&format!("{}{}", prefix_legacy, hash), position);

    let used_attr = if add_used_attribute {
        quote! { #[used] }
    } else {
        quote! {}
    };

    quote! {
        // We bundle both the legacy and new link sections for compatibility.
        static __LEGACY_LINK_SECTION: &'static [u8] = {
            const __BUFFER: #buffer_type_legacy = #serialize_fn_legacy(&#item);
            const __BYTES: &[u8] = __BUFFER.as_ref();
            const __LEN: usize = __BYTES.len();

            #used_attr
            #[unsafe(export_name = #legacy_export_name)]
            static __LINK_SECTION: [u8; __LEN] = #copy_bytes_fn(__BYTES);
            &__LINK_SECTION
        };

        static __LINK_SECTION: &'static [u8] = {
            const __BUFFER: #buffer_type_current = #serialize_fn_current(&#item);
            const __BYTES: &[u8] = __BUFFER.as_ref();
            const __LEN: usize = __BYTES.len();

            #used_attr
            #[unsafe(export_name = #export_name)]
            static __LINK_SECTION: [u8; __LEN] = #copy_bytes_fn(__BYTES);
            &__LINK_SECTION
        };
    }
}
