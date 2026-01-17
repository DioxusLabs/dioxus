use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;

/// We store description of the assets an application uses in the executable.
/// We use the `link_section` attribute embed an extra section in the executable.
/// We force rust to store a serialized representation of the asset description
/// inside a particular region of the binary, with the label "manganis".
/// After linking, the "manganis" sections of the different object files will be merged.
pub fn generate_link_section(asset: impl ToTokens, asset_hash: &str) -> TokenStream2 {
    let position = proc_macro2::Span::call_site();
    let export_name = syn::LitStr::new(&format!("__ASSETS__{}", asset_hash), position);
    let legacy_export_name = syn::LitStr::new(&format!("__MANGANIS__{}", asset_hash), position);

    quote::quote! {
        // We bundle both the legacy and new link sections for compatibility with older CLIs
        static __LEGACY_LINK_SECTION: &'static [u8] = {
            // First serialize the asset into a constant sized buffer
            const __BUFFER: manganis::macro_helpers::const_serialize_07::ConstVec<u8> = manganis::macro_helpers::serialize_asset_07(&#asset);
            // Then pull out the byte slice
            const __BYTES: &[u8] = __BUFFER.as_ref();
            // And the length of the byte slice
            const __LEN: usize = __BYTES.len();

            // Now that we have the size of the asset, copy the bytes into a static array
            #[unsafe(export_name = #legacy_export_name)]
            static __LINK_SECTION: [u8; __LEN]  = manganis::macro_helpers::copy_bytes(__BYTES);
            &__LINK_SECTION
        };

        static __LINK_SECTION: &'static [u8] = {
            // First serialize the asset into a constant sized buffer
            const __BUFFER: manganis::macro_helpers::const_serialize::ConstVec<u8> = manganis::macro_helpers::serialize_asset(&#asset);
            // Then pull out the byte slice
            const __BYTES: &[u8] = __BUFFER.as_ref();
            // And the length of the byte slice
            const __LEN: usize = __BYTES.len();

            // Now that we have the size of the asset, copy the bytes into a static array
            #[unsafe(export_name = #export_name)]
            static __LINK_SECTION: [u8; __LEN]  = manganis::macro_helpers::copy_bytes(__BYTES);
            &__LINK_SECTION
        };
    }
}
