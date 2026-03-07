use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use quote::ToTokens;

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
/// Uses the generic linker helper from dx-macro-helpers for consistency.
pub fn generate_link_section(asset: impl ToTokens, asset_hash: &str) -> TokenStream2 {
    let item = asset;
    let hash: &str = asset_hash;
    let prefix_current: &str = "__ASSETS__";
    let prefix_legacy: &str = "__MANGANIS__";
    let serialize_fn_current = quote! { manganis::macro_helpers::serialize_asset };
    let serialize_fn_legacy = quote! { manganis::macro_helpers::serialize_asset_07 };
    let copy_bytes_fn = quote! { manganis::macro_helpers::copy_bytes };
    let buffer_type_current = quote! { manganis::macro_helpers::ConstVec<u8, 4096> };
    let buffer_type_legacy = quote! { manganis::macro_helpers::const_serialize_07::ConstVec<u8> };
    let position = proc_macro2::Span::call_site();
    let export_name = syn::LitStr::new(&format!("{}{}", prefix_current, hash), position);
    let legacy_export_name = syn::LitStr::new(&format!("{}{}", prefix_legacy, hash), position);

    let used_attr = if false {
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
