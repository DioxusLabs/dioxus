use proc_macro::TokenStream;

use digest::Digest;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, parse_quote, FnArg, Ident, ItemFn, PatIdent, ReturnType, Signature};

/// Annotate a function with `#[hot]` to make it hot-reloadable.
///
/// This can be used on functions and methods. Changes to the assembly "beneath" the function will
/// cause the function to be recompiled and the new assembly to be executed.
///
/// If the changes propagate above the function, the nearest `#[hot]` function will be used as the
/// hot-reload point.
///
/// ```
/// struct Foo {}
///
/// impl Foo {
///     #[hot]
///     fn tick(&mut self) {
///        self.do_stuff()
///     }
/// }
/// ```
///
/// ## Expansion:
///
/// This macro simply expands functions from the following form:
///
/// ```rust
/// #[hot]
/// fn do_thing(a: A, b: B) -> C {
/// }
/// ```
///
/// to the following:
///
/// ```rust
/// fn do_thing(a: A, b: B) -> C {
///     #[inline(never)] // force this as a real symbol
///     fn __hot_do_thing(a: A, b: B) -> C {
///         do_thing_inner(a, b)
///     }
///
///     subsecond::current(do_thing_inner).call((a, b))
/// }
/// ```
///
/// You could also just call `subsecond::current()` yourself, though that interface is slightly
/// unwieldy and intended for use by framework authors.
#[proc_macro_attribute]
pub fn hot(_args: TokenStream, input: TokenStream) -> TokenStream {
    /*
    #[hot]
    fn do_thing(a: A, b: B) -> C {
    }

    // expands to

    fn do_thing(a: A, b: B) -> C {
        #[inline(never)] // force this as a real symbol
        fn __hot_do_thing(a: A, b: B) -> C {
            do_thing_inner(a, b)
        }

        subsecond::current(do_thing_inner).call((a, b))
    }


    // for methods, we don't know the type of the receiver, so we generate another method that's hidden
    // that also takes `self` as an argument
    //
    // note that we want to retain the names of idents so rust-analyzer provides the correct info

    struct Foo {}
    impl Foo {
        #[hot]
        fn do_thing(&self, a: A, b: B) -> C {
            // code...
        }

        // expands to
        fn do_thing(&self, a: A, b: B) -> C {
            subsecond::current(Self::__hot_do_thing).call((self, a, b))
        }

        fn __hot_do_thing(&self, a: A, b: B) -> C {
            // code...
        }
    }
    */

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = parse_macro_input!(input as ItemFn);

    let mut outer_sig = sig.clone();
    let mut inner_sig = sig.clone();
    inner_sig.ident = format_ident!("__hot_{}", sig.ident);

    let inner_fn_name = inner_sig.ident.clone();

    let mut args = vec![];
    for (i, param) in outer_sig.inputs.iter_mut().enumerate() {
        match param {
            syn::FnArg::Receiver(_) => args.push(format_ident!("self")),
            syn::FnArg::Typed(pat_type) => {
                match &*pat_type.pat {
                    // Attempt to preserve original ident for better RA support
                    syn::Pat::Ident(pat_ident) => {
                        args.push(pat_ident.ident.clone());
                    }

                    // Otherwise, generate a new ident
                    _ => {
                        // Create a new ident to tie the outer to the call of the inner
                        let param_ident = format_ident!("__hot_arg_{i}");
                        args.push(param_ident.clone());
                        pat_type.pat = Box::new(syn::Pat::Ident(syn::PatIdent {
                            attrs: vec![],
                            by_ref: None,
                            mutability: None,
                            ident: param_ident,
                            subpat: None,
                        }));
                    }
                }
            }
        }
    }

    let self_ident = if outer_sig
        .inputs
        .first()
        .map(|arg| matches!(arg, FnArg::Receiver(_)))
        == Some(true)
    {
        quote! {  Self:: }
    } else {
        quote! {}
    };

    quote! {
        // the primary function
        // &self, Pattern { a, b, c}: i32, b: i32, c: i32, etc
        // becomes
        // self: &mut Self, arg0: i32, arg1: i32, arg2: i32, etc
        #(#attrs)*
        #vis #outer_sig {
            subsecond::current(#self_ident #inner_fn_name).call(
                (#(#args),*) // .call((self, arg0, arg1))
            )
        }

        // retains the original function signature
        // &self, a: i32, b: i32, c: i32, etc
        #[doc(hidden)]
        #[inline(never)]
        #inner_sig {
            #block
        }
    }
    .into()
}
