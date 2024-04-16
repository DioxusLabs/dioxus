//! Hotreload dynamic nodes, where possible
//!
//! This is limited to formatted strings and literals, since we have an obvious way to serialize them.
//! Eventually we could support any serde type.

use dioxus_core::prelude::Template;
use dioxus_rsx::{
    hot_reload::{diff_rsx, template_location, ChangedRsx, DiffResult, Empty},
    hotreload::HotreloadingResults,
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

fn prettyprint(tokens: TokenStream) -> String {
    let old: CallBody = syn::parse2(tokens).unwrap();
    let as_file: syn::File = syn::parse_quote!(
        fn main() {
            #old
        }
    );
    prettyplease::unparse(&as_file)
}

#[test]
fn tokens_generate_for_formatted_string() {
    let old = quote! {
        div { "asdasd {something} homm {other} else" }
    };

    println!("{}", prettyprint(old));
}

#[test]
fn formatted_strings() {
    let old = quote! {
        div {
            "one {two} three {four} five {six}"
        }
    };

    let new_valid = quote! {
        div {
            "one {two} three {four} five {six} seven!"
        }
    };

    // The new template has a different formatting but can be hotreloaded
    let changed = boilerplate(old.clone(), new_valid).unwrap();
    dbg!(changed);

    let valid_mixed = quote! {
        div {
            "one {two} {four} {six} changeda"
        }
    };

    let changed_mixed = boilerplate(old, valid_mixed).unwrap();
    dbg!(changed_mixed);
}

#[test]
fn formatted_props() {
    let old = quote! {
        Component {
            class: "abc {hidden}"
        }
    };

    let new_valid = quote! {
        Component {
            class: "abc {hidden} def"
        }
    };

    println!("{}", prettyprint(old.clone()));
    // The new template has a different formatting but can be hotreloaded
    let changed = boilerplate(old.clone(), new_valid).unwrap();
    dbg!(changed);

    // static BLAH: &'static str = "asdasd";

    // let v = concat!(BLAH, "asdasd");
}
