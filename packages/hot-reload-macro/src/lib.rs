use proc_macro::TokenStream;
use syn::__private::quote::quote;

#[proc_macro]
pub fn hot_reload(_: TokenStream) -> TokenStream {
    quote!(dioxus_hot_reload::init(core::env!("CARGO_MANIFEST_DIR"))).into()
}
