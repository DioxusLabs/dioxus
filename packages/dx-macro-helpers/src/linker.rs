//! Generic linker section generation for binary embedding
//!
//! This module provides utilities for generating linker sections that embed
//! serialized data in binaries with unique export names.

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};

/// Generate a linker section for embedding serialized data in the binary
///
/// This function creates a static array containing serialized data and exports it
/// with a unique symbol name that can be found by build tools. The exported symbol
/// follows the pattern `{prefix}{hash}` and can be extracted from the binary after linking.
///
/// # Parameters
///
/// - `item`: The item to serialize (must implement `ToTokens`)
/// - `hash`: Unique hash string for the export name
/// - `prefix`: Export prefix (e.g., `"__MY_CRATE__"`)
/// - `serialize_fn`: Path to the serialization function (as a `TokenStream`)
/// - `copy_bytes_fn`: Path to the `copy_bytes` function (as a `TokenStream`)
/// - `buffer_type`: The type of the buffer (e.g., `ConstVec<u8>` or `ConstVec<u8, 4096>`)
/// - `add_used_attribute`: Whether to add the `#[used]` attribute (some crates need it)
///
/// # Example
///
/// ```ignore
/// generate_link_section(
///     my_data,
///     "abc123",
///     "__MY_CRATE__",
///     quote! { my_crate::macro_helpers::serialize_data },
///     quote! { my_crate::macro_helpers::copy_bytes },
///     quote! { my_crate::macro_helpers::const_serialize::ConstVec<u8> },
///     false,
/// )
/// ```
pub fn generate_link_section(
    item: impl ToTokens,
    hash: &str,
    prefix: &str,
    serialize_fn: TokenStream2,
    copy_bytes_fn: TokenStream2,
    buffer_type: TokenStream2,
    add_used_attribute: bool,
) -> TokenStream2 {
    let position = proc_macro2::Span::call_site();
    let export_name = syn::LitStr::new(&format!("{}{}", prefix, hash), position);

    let used_attr = if add_used_attribute {
        quote! { #[used] }
    } else {
        quote! {}
    };

    quote! {
        // First serialize the item into a constant sized buffer
        const __BUFFER: #buffer_type = #serialize_fn(&#item);
        // Then pull out the byte slice
        const __BYTES: &[u8] = __BUFFER.as_ref();
        // And the length of the byte slice
        const __LEN: usize = __BYTES.len();

        // Now that we have the size of the item, copy the bytes into a static array
        #used_attr
        #[unsafe(export_name = #export_name)]
        static __LINK_SECTION: [u8; __LEN] = #copy_bytes_fn(__BYTES);
    }
}

/// Generate a pair of linker sections for legacy + current formats
///
/// This is useful when emitting both old and new symbol formats for compatibility.
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
