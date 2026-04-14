use proc_macro::TokenStream;

use digest::Digest;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_quote, FnArg, Ident, ItemFn, ReturnType, Signature};

#[proc_macro_attribute]
pub fn wasm_split(args: TokenStream, input: TokenStream) -> TokenStream {
    let module_ident = parse_macro_input!(args as Ident);
    let item_fn = parse_macro_input!(input as ItemFn);

    if item_fn.sig.asyncness.is_none() {
        panic!("wasm_split functions must be async. Use a LazyLoader with synchronous functions instead.");
    }

    let LoaderNames {
        split_loader_ident,
        impl_import_ident,
        impl_export_ident,
        load_module_ident,
        ..
    } = LoaderNames::new(item_fn.sig.ident.clone(), module_ident.to_string());

    let mut desugard_async_sig = item_fn.sig.clone();
    desugard_async_sig.asyncness = None;
    desugard_async_sig.output = match &desugard_async_sig.output {
        ReturnType::Default => {
            parse_quote! { -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = ()>>> }
        }
        ReturnType::Type(_, ty) => {
            parse_quote! { -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = #ty>>> }
        }
    };

    let import_sig = Signature {
        ident: impl_import_ident.clone(),
        ..desugard_async_sig.clone()
    };

    let export_sig = Signature {
        ident: impl_export_ident.clone(),
        ..desugard_async_sig.clone()
    };

    let default_item = item_fn.clone();

    let mut wrapper_sig = item_fn.sig;
    wrapper_sig.asyncness = Some(Default::default());

    let mut args = Vec::new();
    for (i, param) in wrapper_sig.inputs.iter_mut().enumerate() {
        match param {
            syn::FnArg::Receiver(_) => args.push(format_ident!("self")),
            syn::FnArg::Typed(pat_type) => {
                let param_ident = format_ident!("__wasm_split_arg_{i}");
                args.push(param_ident.clone());
                *pat_type.pat = syn::Pat::Ident(syn::PatIdent {
                    attrs: vec![],
                    by_ref: None,
                    mutability: None,
                    ident: param_ident,
                    subpat: None,
                });
            }
        }
    }

    let attrs = &item_fn.attrs;
    let stmts = &item_fn.block.stmts;

    quote! {
        #[cfg(target_arch = "wasm32")]
        #wrapper_sig {
            #(#attrs)*
            #[allow(improper_ctypes_definitions)]
            #[no_mangle]
            pub extern "C" #export_sig {
                Box::pin(async move { #(#stmts)* })
            }

            #[link(wasm_import_module = "./__wasm_split.js")]
            extern "C" {
                #[no_mangle]
                fn #load_module_ident (
                    callback: unsafe extern "C" fn(*const ::std::ffi::c_void, bool),
                    data: *const ::std::ffi::c_void
                );

                #[allow(improper_ctypes)]
                #[no_mangle]
                #import_sig;
            }

            thread_local! {
                static #split_loader_ident: wasm_split::LazySplitLoader = unsafe {
                    wasm_split::LazySplitLoader::new(#load_module_ident)
                };
            }

            // Initiate the download by calling the load_module_ident function which will kick-off the loader
            if !wasm_split::LazySplitLoader::ensure_loaded(&#split_loader_ident).await {
                panic!("Failed to load wasm-split module");
            }

            unsafe { #impl_import_ident( #(#args),* ) }.await
        }

        #[cfg(not(target_arch = "wasm32"))]
        #default_item
    }
    .into()
}

/// Create a lazy loader for a given function. Meant to be used in statics. Designed for libraries to
/// integrate with.
///
/// ```rust, ignore
/// fn SomeFunction(args: Args) -> Ret {}
///
/// static LOADER: wasm_split::LazyLoader<Args, Ret> = lazy_loader!(SomeFunction);
///
/// LOADER.load().await.call(args)
/// ```
#[proc_macro]
pub fn lazy_loader(input: TokenStream) -> TokenStream {
    // We can only accept idents/paths that will be the source function
    let sig = parse_macro_input!(input as Signature);
    let params = sig.inputs.clone();
    let outputs = sig.output.clone();
    let Some(FnArg::Typed(arg)) = params.first().cloned() else {
        panic!(
            "Lazy Loader must define a single input argument to satisfy the LazyLoader signature"
        )
    };
    let arg_ty = arg.ty.clone();
    let LoaderNames {
        name,
        split_loader_ident,
        impl_import_ident,
        impl_export_ident,
        load_module_ident,
        ..
    } = LoaderNames::new(
        sig.ident.clone(),
        sig.abi
            .as_ref()
            .and_then(|abi| abi.name.as_ref().map(|f| f.value()))
            .expect("abi to be module name")
            .to_string(),
    );

    quote! {
        {
            #[cfg(target_arch = "wasm32")]
            {
                #[link(wasm_import_module = "./__wasm_split.js")]
                extern "C" {
                    // The function we'll use to initiate the download of the module
                    #[no_mangle]
                    fn #load_module_ident(
                        callback: unsafe extern "C" fn(*const ::std::ffi::c_void, bool),
                        data: *const ::std::ffi::c_void,
                    );

                    #[allow(improper_ctypes)]
                    #[no_mangle]
                    fn #impl_import_ident(arg: #arg_ty) #outputs;
                }


                #[allow(improper_ctypes_definitions)]
                #[no_mangle]
                pub extern "C" fn #impl_export_ident(arg: #arg_ty) #outputs {
                    #name(arg)
                }

                thread_local! {
                    static #split_loader_ident: wasm_split::LazySplitLoader = unsafe {
                        wasm_split::LazySplitLoader::new(#load_module_ident)
                    };
                };

                unsafe {
                    wasm_split::LazyLoader::new(#impl_import_ident, &#split_loader_ident)
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                wasm_split::LazyLoader::preloaded(#name)
            }
        }
    }
    .into()
}

struct LoaderNames {
    name: Ident,
    split_loader_ident: Ident,
    impl_import_ident: Ident,
    impl_export_ident: Ident,
    load_module_ident: Ident,
}

impl LoaderNames {
    fn new(name: Ident, module: String) -> Self {
        let unique_identifier = base16::encode_lower(
            &sha2::Sha256::digest(format!("{name} {span:?}", name = name, span = name.span()))
                [..16],
        );

        Self {
            split_loader_ident: format_ident!("__wasm_split_loader_{module}"),
            impl_export_ident: format_ident!(
                "__wasm_split_00___{module}___00_export_{unique_identifier}_{name}"
            ),
            impl_import_ident: format_ident!(
                "__wasm_split_00___{module}___00_import_{unique_identifier}_{name}"
            ),
            load_module_ident: format_ident!(
                "__wasm_split_load_{module}_{unique_identifier}_{name}"
            ),
            name,
        }
    }
}
