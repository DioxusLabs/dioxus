use proc_macro::TokenStream;

use digest::Digest;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, Ident, ItemFn, Pat, Signature};

#[proc_macro_attribute]
pub fn wasm_split(args: TokenStream, input: TokenStream) -> TokenStream {
    let module_ident = parse_macro_input!(args as Ident);
    let item_fn = parse_macro_input!(input as ItemFn);

    let name = &item_fn.sig.ident;

    let unique_identifier = base16::encode_lower(
        &sha2::Sha256::digest(format!("{name} {span:?}", span = name.span()))[..16],
    );

    // let load_module_ident = format_ident!("__wasm_split_load_{module_ident}");
    let load_module_ident =
        format_ident!("__wasm_split_load_{module_ident}_{unique_identifier}_{name}");

    let split_loader_ident = format_ident!("__wasm_split_loader");
    let impl_import_ident =
        format_ident!("__wasm_split_00___{module_ident}___00_import_{unique_identifier}_{name}");
    let impl_export_ident =
        format_ident!("__wasm_split_00___{module_ident}___00_export_{unique_identifier}_{name}");

    let import_sig = Signature {
        ident: impl_import_ident.clone(),
        asyncness: None,
        ..item_fn.sig.clone()
    };
    let export_sig = Signature {
        ident: impl_export_ident.clone(),
        asyncness: None,
        ..item_fn.sig.clone()
    };

    let mut wrapper_sig = item_fn.sig;
    wrapper_sig.asyncness = Some(Default::default());

    let mut args = Vec::new();
    for (i, param) in wrapper_sig.inputs.iter_mut().enumerate() {
        match param {
            syn::FnArg::Typed(pat_type) => {
                let param_ident = format_ident!("__wasm_split_arg_{i}");
                args.push(param_ident.clone());
                pat_type.pat = Box::new(syn::Pat::Ident(syn::PatIdent {
                    attrs: vec![],
                    by_ref: None,
                    mutability: None,
                    ident: param_ident,
                    subpat: None,
                }));
            }
            syn::FnArg::Receiver(_) => {
                args.push(format_ident!("self"));
            }
        }
    }

    let attrs = item_fn.attrs;

    let stmts = &item_fn.block.stmts;

    quote! {
        #wrapper_sig {
            #(#attrs)*
            #[allow(improper_ctypes_definitions)]
            #[no_mangle]
            pub extern "C" #export_sig {
                #(#stmts)*
            }

            #[link(wasm_import_module = "./__wasm_split.js")]
            extern "C" {
                // The function we'll use to initiate the download of the module
                // The callback passed here
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
                static #split_loader_ident: dioxus::wasm_split::LazySplitLoader = unsafe {
                    dioxus::wasm_split::LazySplitLoader::new(#load_module_ident)
                };
            }

            // Initiate the download by calling the load_module_ident function which will kick-off the loader
            let res = dioxus::wasm_split::ensure_loaded(&#split_loader_ident).await;
            if !res {
                panic!("Failed to load wasm-split module");
            }

            unsafe { #impl_import_ident( #(#args),* ) }
        }
    }
    .into()
}

/// Create a lazy loader for a given function. Meant to be used in statics. Designed for libraries to
/// integrate with.
///
/// ```rust, no_run
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
    let name = sig.ident.clone();
    let params = sig.inputs.clone();
    let outputs = sig.output.clone();
    let arg = params.first().cloned().unwrap();
    let FnArg::Typed(arg) = arg else {
        panic!("Lazy Loader must define a single argument")
    };
    let Pat::Ident(arg_name) = &*arg.pat else {
        panic!("Lazy Loader must define a single argument")
    };
    let arg_ty = arg.ty.clone();

    let name = &sig.ident;
    let unique_identifier = base16::encode_lower(
        &sha2::Sha256::digest(format!("{name} {span:?}", span = name.span()))[..16],
    );

    let module_ident = sig
        .abi
        .as_ref()
        .and_then(|abi| abi.name.as_ref().map(|f| f.value()))
        .unwrap_or_else(|| {
            panic!("needs abi");
        });

    // let load_module_ident = format_ident!("__wasm_split_load_{module_ident}");
    // let load_module_ident = format_ident!("__wasm_split_load_{module_ident}");
    let load_module_ident =
        format_ident!("__wasm_split_load_{module_ident}_{unique_identifier}_{name}");

    let split_loader_ident = format_ident!("__wasm_split_loader_{module_ident}");
    let impl_import_ident =
        format_ident!("__wasm_split_00___{module_ident}___00_import_{unique_identifier}_{name}");
    let impl_export_ident =
        format_ident!("__wasm_split_00___{module_ident}___00_export_{unique_identifier}_{name}");

    quote! {
        {
            #[link(wasm_import_module = "./__wasm_split.js")]
            extern "C" {
                // The function we'll use to initiate the download of the module
                // The callback passed here
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
                static #split_loader_ident: dioxus::wasm_split::LazySplitLoader = unsafe {
                    dioxus::wasm_split::LazySplitLoader::new(#load_module_ident)
                };
            };

            dioxus::wasm_split::LazyLoader {
                key: &#split_loader_ident,
                imported: #impl_import_ident,
                loaded: std::sync::atomic::AtomicBool::new(false),
            }
        }
    }
    .into()
}
