use core::panic;
use manganis_core::ResourceAsset;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned, ToTokens};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, path::Path, sync::atomic::AtomicBool};
use std::{path::PathBuf, sync::atomic::Ordering};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Token,
    Expr, ExprLit, Lit, LitStr, PatLit, Token,
};

pub struct MethodCalls {
    pub options: Vec<MethodCallOption>,
}

/// A builder method in the form of `.method(arg1, arg2)`
pub struct MethodCallOption {
    pub method: syn::Ident,
    pub args: Punctuated<syn::Lit, Token![,]>,
}

impl MethodCalls {
    // fn new(args: Vec<MethodCallOption>) -> Option<FileOptions> {
    //     let asset_type = args.first()?.method.to_string();

    //     let stack = args
    //         .into_iter()
    //         .skip(1)
    //         .map(|x| (x.method.to_string(), x.args.into_iter().collect::<Vec<_>>()))
    //         .collect::<HashMap<String, Vec<syn::Lit>>>();

    // let opts = match asset_type.as_str() {
    //     "image" => {
    //         let mut opts = ImageOptions::new(manganis_common::ImageType::Avif, Some((32, 32)));
    //         // opts.set_preload(preload);
    //         // opts.set_url_encoded(url_encoded);
    //         // opts.set_low_quality_preview(low_quality_preview);
    //         FileOptions::Image(opts)
    //     }

    //     "video" => FileOptions::Video(VideoOptions::new(todo!())),
    //     "font" => FileOptions::Font(FontOptions::new(todo!())),
    //     "css" => FileOptions::Css(CssOptions::new()),
    //     "js" => FileOptions::Js(JsOptions::new(todo!())),
    //     "json" => FileOptions::Json(JsonOptions::new()),
    //     other => FileOptions::Other(UnknownFileOptions::new(todo!())),
    // };

    // Some(opts)
    //     None
    // }
}

// let local = match asset.local.as_ref() {
//     Some(local) => {
//         let local = local.display().to_string();
//         quote! { #local }
//     }
//     None => {
//         todo!("relative paths are not supported yet")
//         // quote! {
//         //     {
//         //         // ensure it exists by throwing away the include_bytes
//         //         static _BLAH: &[u8] = include_bytes!(#input);
//         //         // But then pass along the path
//         //         concat!(env!("CARGO_MANIFEST_DIR"), "/", file!(), "/<split>/", #input)
//         //     }
//         // }
//     }
// };
