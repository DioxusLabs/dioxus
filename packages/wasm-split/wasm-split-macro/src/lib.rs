use proc_macro::TokenStream;

use digest::Digest;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Ident, ItemFn, Signature};

#[proc_macro_attribute]
pub fn wasm_split(args: TokenStream, input: TokenStream) -> TokenStream {
    let module_ident = parse_macro_input!(args as Ident);
    let item_fn = parse_macro_input!(input as ItemFn);

    let name = &item_fn.sig.ident;

    let unique_identifier = base16::encode_lower(
        &sha2::Sha256::digest(format!("{name} {span:?}", span = name.span()))[..16],
    );

    let load_module_ident = format_ident!("__wasm_split_load_{module_ident}");

    let split_loader_ident = format_ident!("__wasm_split_loader");
    let impl_import_ident =
        format_ident!("__wasm_split_00{module_ident}00_import_{unique_identifier}_{name}");
    let impl_export_ident =
        format_ident!("__wasm_split_00{module_ident}00_export_{unique_identifier}_{name}");

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
            thread_local! {
                static #split_loader_ident: ::wasm_split::LazySplitLoader = unsafe {
                    ::wasm_split::LazySplitLoader::new(#load_module_ident)
                };
            }

            #[link(wasm_import_module = "./__wasm_split.js")]
            extern "C" {
                // The function we'll use to initiate the download of the module
                // The callback passed here
                #[no_mangle]
                fn #load_module_ident (
                    callback: unsafe extern "C" fn(*const ::std::ffi::c_void, bool),
                    data: *const ::std::ffi::c_void
                ) -> ();

                #[allow(improper_ctypes)]
                #[no_mangle]
                #import_sig;
            }

            #(#attrs)*
            #[allow(improper_ctypes_definitions)]
            #[no_mangle]
            pub extern "C" #export_sig {
                #(#stmts)*
            }

            // Initiate the download by calling the load_module_ident function which will kick-off the loader
            let load = ::wasm_split::ensure_loaded(&#split_loader_ident).await;

            web_sys::console::log_1(&"loader called; ".into());


            // // Now actually call the imported function
            if load.is_some() {
                web_sys::console::log_1(&"loader has data; ".into());
                let res = unsafe { #impl_import_ident( #(#args),* ) };
            }


            web_sys::console::log_1(&"loader returned; ".into());

        }
    }
    .into()
}
