//! Attribute macro stubs for the inspector runtime.

use proc_macro::TokenStream;

/// Placeholder attribute so existing `#[inspector]` usages continue to compile.
#[proc_macro_attribute]
pub fn inspector(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}
