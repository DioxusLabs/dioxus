use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{
    parse::{Parse, ParseStream},
    Signature,
};
use syn::{
    parse_macro_input, Attribute, Block, FnArg, Ident, Item, ItemFn, ReturnType, Type, Visibility,
};

mod fc;
mod ifmt;

/// Label a function or static closure as a functional component.
/// This macro reduces the need to create a separate properties struct.
#[proc_macro_attribute]
pub fn fc(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use fc::{function_component_impl, FunctionComponent};

    let item = parse_macro_input!(item as FunctionComponent);

    function_component_impl(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()

    // function_component_impl(attr, item)
    // let attr = parse_macro_input!(attr as FunctionComponentName);
}

#[proc_macro]
pub fn format_args_f(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use ifmt::*;

    let item = parse_macro_input!(input as IfmtInput);

    // #[allow(unused)]
    // const FUNCTION_NAME: &str = "format_args_f";

    // debug_input!(&input);

    ifmt::format_args_f_impl(item)
    // .unwrap_or_else(|err| err.to_compile_error())
    // .into()
}
