#![allow(unused)]

use dioxus_rsx::{
    hot_reload::{diff_rsx, template_location, ChangedRsx, DiffResult},
    CallBody, HotReloadingContext,
};
use quote::quote;
use syn::{parse::Parse, spanned::Spanned, File};

#[derive(Debug)]
struct Mock;

impl HotReloadingContext for Mock {
    fn map_attribute(
        element_name_rust: &str,
        attribute_name_rust: &str,
    ) -> Option<(&'static str, Option<&'static str>)> {
        match element_name_rust {
            "svg" => match attribute_name_rust {
                "width" => Some(("width", Some("style"))),
                "height" => Some(("height", Some("style"))),
                _ => None,
            },
            _ => None,
        }
    }

    fn map_element(element_name_rust: &str) -> Option<(&'static str, Option<&'static str>)> {
        match element_name_rust {
            "svg" => Some(("svg", Some("svg"))),
            _ => None,
        }
    }
}
#[test]
fn testing_for_pattern() {
    let old = quote! {
        div {
            for item in vec![1, 2, 3] {
                div { "123" }
                div { "asasddasdasd" }
            }
        }
    };

    let new = quote! {
        div {
            for item in vec![1, 2, 3] {
                div { "asasddasdasd" }
            }
        }
    };

    let old: CallBody = syn::parse2(old).unwrap();
    let new: CallBody = syn::parse2(new).unwrap();

    let updated = old.update_template::<Mock>(Some(new), "testing");

    // currently, modifying a for loop is not hot reloadable
    // We want to change this...
    assert!(updated.is_none());

    // let updated = old.update_template::<Mock>(Some(new), "testing").unwrap();

    // let old = include_str!(concat!("./valid/for_.old.rsx"));
    // let new = include_str!(concat!("./valid/for_.new.rsx"));
    // let (old, new) = load_files(old, new);

    // let DiffResult::RsxChanged { rsx_calls } = diff_rsx(&new, &old) else {
    //     panic!("Expected a rsx call to be changed")
    // };

    // for calls in rsx_calls {
    //     let ChangedRsx { old, new } = calls;

    //     let old_start = old.span().start();

    //     let old_call_body = syn::parse2::<CallBody>(old.tokens).unwrap();
    //     let new_call_body = syn::parse2::<CallBody>(new).unwrap();

    //     let leaked_location = Box::leak(template_location(old_start, file).into_boxed_str());

    //     let hotreloadable_template =
    //         new_call_body.update_template::<Ctx>(Some(old_call_body), leaked_location);

    //     dbg!(hotreloadable_template);
    // }

    // dbg!(rsx_calls);
}
