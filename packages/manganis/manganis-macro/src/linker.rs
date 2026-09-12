use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::quote;

/// The size of every linker section emitted by manganis. Serialization pads to this size.
const LINK_SECTION_SIZE: usize = 4096;

/// Generate a linker section for embedding arbitrary data in the binary
///
/// This is a generic version that allows customizing the serialization function.
/// Used by both asset and FFI metadata embedding. `serialize_fn` must return a full
/// `ConstVec<u8, 4096>`.
pub fn generate_link_section_inner(
    item: TokenStream2,
    hash: &str,
    prefix: &str,
    serialize_fn: TokenStream2,
) -> TokenStream2 {
    let position = proc_macro2::Span::call_site();
    let export_name = syn::LitStr::new(&format!("{}{}", prefix, hash), position);

    quote! {
        #[used]
        static __LINK_SECTION: &'static [u8] = {
            #[unsafe(export_name = #export_name)]
            #[used]
            static __LINK_SECTION: [u8; #LINK_SECTION_SIZE] = #serialize_fn(&#item).into_array();
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
    let position = proc_macro2::Span::call_site();
    let export_name = syn::LitStr::new(&format!("__ASSETS__{}", asset_hash), position);

    quote! {
        static __LINK_SECTION: &'static [u8] = {
            #[unsafe(export_name = #export_name)]
            static __LINK_SECTION: [u8; #LINK_SECTION_SIZE] =
                manganis::macro_helpers::serialize_asset(&#item).into_array();
            &__LINK_SECTION
        };
    }
}
