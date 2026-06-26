use proc_macro::TokenStream;

use quote::ToTokens;
use syn::parse_macro_input;

mod common;
mod elements;
mod events;
mod extension_attributes;

use elements::DefineElements;
use events::EventExtensions;
use extension_attributes::ImplExtensionAttributes;

#[proc_macro]
pub fn impl_extension_attributes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ImplExtensionAttributes);
    input.to_token_stream().into()
}

/// Generate typed element constructors and typed attribute extension traits.
#[proc_macro]
pub fn define_elements(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DefineElements);
    input.to_token_stream().into()
}

/// Generate the `EventsExtension` trait that adds event handler methods to typed HTML builders.
///
/// Each entry has the form `#[attrs] method_name => raw_event => DataType,` where `method_name`
/// is the builder method (e.g. `onclick`), `raw_event` is the DOM event name without the `on`
/// prefix (e.g. `click`), and `DataType` is the typed event data (e.g. `MouseData`).
#[proc_macro]
pub fn impl_event_extensions(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as EventExtensions);
    input.to_token_stream().into()
}
