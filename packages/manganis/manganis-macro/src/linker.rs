use proc_macro2::TokenStream as TokenStream2;
use serde::Serialize;

/// this new approach will store the assets descriptions *inside the executable*.
/// The trick is to use the `link_section` attribute.
/// We force rust to store a json representation of the asset description
/// inside a particular region of the binary, with the label "manganis".
/// After linking, the "manganis" sections of the different executables will be merged.
pub fn generate_link_section(asset: &impl Serialize) -> TokenStream2 {
    let position = proc_macro2::Span::call_site();
    let asset_description = serde_json::to_string(asset).unwrap();
    let len = asset_description.as_bytes().len();
    let asset_bytes = syn::LitByteStr::new(asset_description.as_bytes(), position);
    let section_name = syn::LitStr::new(manganis_core::LinkSection::CURRENT.link_section, position);

    quote::quote! {
        #[link_section = #section_name]
        #[used]
        static ASSET: [u8; #len] = * #asset_bytes;
    }
}
