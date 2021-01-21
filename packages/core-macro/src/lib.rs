use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{
    parse::{Parse, ParseStream},
    Signature,
};
use syn::{
    parse_macro_input, Attribute, Block, FnArg, Ident, Item, ItemFn, ReturnType, Type, Visibility,
};

/// Label a function or static closure as a functional component.
/// This macro reduces the need to create a separate properties struct.
#[proc_macro_attribute]
pub fn fc(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as FunctionComponent);
    // let attr = parse_macro_input!(attr as FunctionComponentName);

    function_component_impl(item)
        // function_component_impl(attr, item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

fn function_component_impl(
    // name: FunctionComponentName,
    component: FunctionComponent,
) -> syn::Result<TokenStream> {
    // let FunctionComponentName { component_name } = name;

    let FunctionComponent {
        block,
        props_type,
        arg,
        vis,
        attrs,
        name: function_name,
        return_type,
    } = component;

    // if function_name == component_name {
    //     return Err(syn::Error::new_spanned(
    //         component_name,
    //         "the component must not have the same name as the function",
    //     ));
    // }

    let ret_type = quote_spanned!(return_type.span()=> VNode);
    // let ret_type = quote_spanned!(return_type.span()=> ::VNode);
    // let ret_type = quote_spanned!(return_type.span()=> ::yew::html::Html);

    let quoted = quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        mod __component_blah {
            use super::*;

            #[derive(PartialEq)]
            pub struct Props {
                name: String
            }

            pub fn component(ctx: &mut Context<Props>) -> #ret_type {
                let Props {
                    name
                } = ctx.props;
                #block
            }
        }
        #[allow(non_snake_case)]
        pub use __component_blah::component as #function_name;




        // #vis struct #function_name;

        // impl ::yew_functional::FunctionProvider for #function_name {
        //     type TProps = #props_type;

        //     fn run(#arg) -> #ret_type {
        //         #block
        //     }
        // }

        // #(#attrs)*
        // #vis type #component_name = ::yew_functional::FunctionComponent<#function_name>;
    };
    // let quoted = quote! {
    //     #[doc(hidden)]
    //     #[allow(non_camel_case_types)]
    //     #vis struct #function_name;

    //     impl ::yew_functional::FunctionProvider for #function_name {
    //         type TProps = #props_type;

    //         fn run(#arg) -> #ret_type {
    //             #block
    //         }
    //     }

    //     #(#attrs)*
    //     #vis type #component_name = ::yew_functional::FunctionComponent<#function_name>;
    // };
    Ok(quoted)
}

/// A parsed version of the user's input
struct FunctionComponent {
    // The actual contents of the function
    block: Box<Block>,

    // The user's props type
    props_type: Box<Type>,

    arg: FnArg,
    vis: Visibility,
    attrs: Vec<Attribute>,
    name: Ident,
    return_type: Box<Type>,
}

impl Parse for FunctionComponent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let parsed: Item = input.parse()?;

        // Convert the parsed input into the Function block
        let ItemFn {
            attrs,
            vis,
            sig,
            block,
        } = ensure_fn_block(parsed)?;

        // Validate the user's signature
        let sig = validate_signature(sig)?;

        // Validate the return type is actually something
        let return_type = ensure_return_type(sig.output)?;

        // Get all the function args
        let mut inputs = sig.inputs.into_iter();

        // Collect the first arg
        let first_arg: FnArg = inputs
            .next()
            .unwrap_or_else(|| syn::parse_quote! { _: &() });

        // Extract the "context" object
        let props_type = validate_context_arg(&first_arg)?;

        // Collect the rest of the args into a list of definitions to be used by the inline struct

        // Checking after param parsing may make it a little inefficient
        // but that's a requirement for better error messages in case of receivers
        // `>0` because first one is already consumed.
        // if inputs.len() > 0 {
        //     let params: TokenStream = inputs.map(|it| it.to_token_stream()).collect();
        //     return Err(syn::Error::new_spanned(
        //         params,
        //         "function components can accept at most one parameter for the props",
        //     ));
        // }
        let name = sig.ident;

        Ok(Self {
            props_type,
            block,
            arg: first_arg,
            vis,
            attrs,
            name,
            return_type,
        })
    }
}

/// Ensure the user's input is actually a functional component
fn ensure_fn_block(item: Item) -> syn::Result<ItemFn> {
    match item {
        Item::Fn(it) => Ok(it),
        other => Err(syn::Error::new_spanned(
            other,
            "`function_component` attribute can only be applied to functions",
        )),
    }
}

/// Ensure the user's function actually returns a VNode
fn ensure_return_type(output: ReturnType) -> syn::Result<Box<Type>> {
    match output {
        ReturnType::Default => Err(syn::Error::new_spanned(
            output,
            "function components must return `dioxus::VNode`",
        )),
        ReturnType::Type(_, ty) => Ok(ty),
    }
}

/// Validate the users's input signature for the function component.
/// Returns an error if any of the conditions prove to be wrong;
fn validate_signature(sig: Signature) -> syn::Result<Signature> {
    if !sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            sig.generics,
            "function components can't contain generics",
        ));
    }

    if sig.asyncness.is_some() {
        return Err(syn::Error::new_spanned(
            sig.asyncness,
            "function components can't be async",
        ));
    }

    if sig.constness.is_some() {
        return Err(syn::Error::new_spanned(
            sig.constness,
            "const functions can't be function components",
        ));
    }

    if sig.abi.is_some() {
        return Err(syn::Error::new_spanned(
            sig.abi,
            "extern functions can't be function components",
        ));
    }

    Ok(sig)
}

fn validate_context_arg(first_arg: &FnArg) -> syn::Result<Box<Type>> {
    if let FnArg::Typed(arg) = first_arg {
        // Input arg is a reference to an &mut Context
        if let Type::Reference(ty) = &*arg.ty {
            if ty.lifetime.is_some() {
                return Err(syn::Error::new_spanned(
                    &ty.lifetime,
                    "reference must not have a lifetime",
                ));
            }

            if ty.mutability.is_some() {
                return Err(syn::Error::new_spanned(
                    &ty.mutability,
                    "reference must not be mutable",
                ));
            }

            Ok(ty.elem.clone())
        } else {
            let msg = format!(
                "expected a reference to a `Context` object (try: `&mut {}`)",
                arg.ty.to_token_stream()
            );
            return Err(syn::Error::new_spanned(arg.ty.clone(), msg));
        }
    } else {
        return Err(syn::Error::new_spanned(
            first_arg,
            "function components can't accept a receiver",
        ));
    }
}

fn collect_inline_args() {}

/// The named specified in the macro usage.
struct FunctionComponentName {
    component_name: Ident,
}

impl Parse for FunctionComponentName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Err(input.error("expected identifier for the component"));
        }

        let component_name = input.parse()?;

        Ok(Self { component_name })
    }
}
