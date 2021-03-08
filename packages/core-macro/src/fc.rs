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

/// A parsed version of the user's input
pub struct FunctionComponent {
    // The actual contents of the function
    block: Box<Block>,

    // // The user's props type
    // props_type: Box<Type>,
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
        // let props_type = validate_context_arg(&first_arg)?;

        /*
        Extract the rest of the function arguments into a struct body
        We require all inputs are strongly typed with names so we can destructure into the function body when expanded


        */
        // let rest = inputs
        //     .map(|f| {
        //         //
        //         match f {
        //             FnArg::Typed(pat) => {
        //                 match *pat.pat {
        //                     syn::Pat::Type(asd) => {}
        //                     _ => {}
        //                 };
        //                 //
        //             }
        //             FnArg::Receiver(_) => {}
        //         }
        //         // let name = f
        //         let stream = f.into_token_stream();
        //         (stream)
        //     })
        //     .collect::<Vec<_>>();

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
            // props_type,
            block,
            arg: first_arg,
            vis,
            attrs,
            name,
            return_type,
        })
    }
}
impl ToTokens for FunctionComponent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // let FunctionComponentName { component_name } = name;

        let FunctionComponent {
            block,
            // props_type,
            arg,
            vis,
            attrs,
            name: function_name,
            return_type,
        } = self;

        // if function_name == component_name {
        //     return Err(syn::Error::new_spanned(
        //         component_name,
        //         "the component must not have the same name as the function",
        //     ));
        // }

        let quoted = quote! {


            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #[derive(PartialEq)]
            pub struct #function_name<'a> {
                // and some other attrs
                ___p: std::marker::PhantomData<&'a ()>
            }

            impl<'a> FC for #function_name<'a> {
                fn render(ctx: Context<'_>, props: &#function_name<'a>) -> DomTree {
                    let #function_name {
                        ..
                    } = props;

                    #block
                }
            }

            // mod __component_blah {
            // use super::*;

            // #[derive(PartialEq)]
            // pub struct Props<'a> {
            //     name: &'a str
            // }

            // pub fn component<'a>(ctx: &'a Context<'a, Props>) -> VNode<'a> {
            //     // Destructure the props into the parent scope
            //     // todo: handle expansion of lifetimes
            //     let Props {
            //         name
            //     } = ctx.props;

            //     #block
            // }
            // }
            // #[allow(non_snake_case)]
            // pub use __component_blah::component as #function_name;
        };

        quoted.to_tokens(tokens);
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
    }
}

/// Ensure the user's input is actually a functional component
pub fn ensure_fn_block(item: Item) -> syn::Result<ItemFn> {
    match item {
        Item::Fn(it) => Ok(it),
        Item::Static(it) => {
            let syn::ItemStatic {
                attrs,
                vis,
                static_token,
                mutability,
                ident,
                colon_token,
                ty,
                eq_token,
                expr,
                semi_token,
            } = &it;
            match ty.as_ref() {
                Type::BareFn(bare) => {}
                // Type::Array(_)
                // | Type::Group(_)
                // | Type::ImplTrait(_)
                // | Type::Infer(_)
                // | Type::Macro(_)
                // | Type::Never(_)
                // | Type::Paren(_)
                // | Type::Path(_)
                // | Type::Ptr(_)
                // | Type::Reference(_)
                // | Type::Slice(_)
                // | Type::TraitObject(_)
                // | Type::Tuple(_)
                // | Type::Verbatim(_)
                _ => {}
            };

            // TODO: Add support for static block
            // Ensure that the contents of the static block can be extracted to a function
            // TODO: @Jon
            // Decide if statics should be converted to functions (under the hood) or stay as statics
            // They _do_ get promoted, but also have a &'static ref
            Err(syn::Error::new_spanned(
                it,
                "`function_component` attribute not ready for statics",
            ))
        }
        other => Err(syn::Error::new_spanned(
            other,
            "`function_component` attribute can only be applied to functions",
        )),
    }
}

/// Ensure the user's function actually returns a VNode
pub fn ensure_return_type(output: ReturnType) -> syn::Result<Box<Type>> {
    match output {
        ReturnType::Default => Err(syn::Error::new_spanned(
            output,
            "function components must return a `DomTree`",
        )),
        ReturnType::Type(_, ty) => Ok(ty),
    }
}

/// Validate the users's input signature for the function component.
/// Returns an error if any of the conditions prove to be wrong;
pub fn validate_signature(sig: Signature) -> syn::Result<Signature> {
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

// pub fn validate_context_arg(first_arg: &FnArg) -> syn::Result<Box<Type>> {
//     if let FnArg::Typed(arg) = first_arg {
//         // if let Type::R
//         // Input arg is a reference to an &mut Context
//         // if let Type::Reference(ty) = &*arg.ty {
//         //     if ty.lifetime.is_some() {
//         //         return Err(syn::Error::new_spanned(
//         //             &ty.lifetime,
//         //             "reference must not have a lifetime",
//         //         ));
//         //     }

//         //     if ty.mutability.is_some() {
//         //         return Err(syn::Error::new_spanned(
//         //             &ty.mutability,
//         //             "reference must not be mutable",
//         //         ));
//         //     }

//         //     Ok(ty.elem.clone())
//         // } else {
//         //     let msg = format!(
//         //         "expected a reference to a `Context` object (try: `&mut {}`)",
//         //         arg.ty.to_token_stream()
//         //     );
//         //     return Err(syn::Error::new_spanned(arg.ty.clone(), msg));
//         // }
//     } else {
//         return Err(syn::Error::new_spanned(
//             first_arg,
//             "function components can't accept a receiver",
//         ));
//     }
// }

pub fn collect_inline_args() {}

/// The named specified in the macro usage.
pub struct FunctionComponentName {
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
