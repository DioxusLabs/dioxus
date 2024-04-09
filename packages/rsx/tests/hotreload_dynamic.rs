//! Hotreload dynamic nodes, where possible
//!
//! This is limited to formatted strings and literals, since we have an obvious way to serialize them.
//! Eventually we could support any serde type.

use dioxus_core::prelude::Template;
use dioxus_rsx::{
    hot_reload::{diff_rsx, template_location, ChangedRsx, DiffResult, Empty},
    tracked::HotreloadingResults,
    CallBody, HotReloadingContext,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse::Parse, spanned::Spanned, token::Token, File};

fn boilerplate(old: TokenStream, new: TokenStream) -> Option<HotreloadingResults> {
    let old: CallBody = syn::parse2(old).unwrap();
    let new: CallBody = syn::parse2(new).unwrap();

    let location = "file:line:col:0";
    hotreload_callbody::<Empty>(&old, &new, location)
}

fn hotreload_callbody<Ctx: HotReloadingContext>(
    old: &CallBody,
    new: &CallBody,
    location: &'static str,
) -> Option<HotreloadingResults> {
    let results = HotreloadingResults::new::<Ctx>(old, new, location)?;
    Some(results)
}

#[test]
fn tokens_generate_for_formatted_string() {
    let old = quote! {
        div { "asdasd {something}" }
    };

    let old: CallBody = syn::parse2(old).unwrap();
    let as_file: syn::File = syn::parse_quote!(
        fn main() {
            #old
        }
    );

    println!("{}", prettyplease::unparse(&as_file));
}

#[test]
fn formatted_strings() {
    let old = quote! {
        div {
            "asdasd {something}"
        }
    };

    let new_valid = quote! {
        div {
            "asdasd {something} else"
        }
    };

    // The new template has a different formatting but can be hotreloaded
    let changed = boilerplate(old, new_valid).unwrap();
    dbg!(changed);
}
