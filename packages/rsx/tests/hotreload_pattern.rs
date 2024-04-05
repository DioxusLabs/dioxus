#![allow(unused)]

use dioxus_rsx::{
    hot_reload::{diff_rsx, template_location, ChangedRsx, DiffResult},
    tracked::hotreload_callbody,
    CallBody, HotReloadingContext,
};
use quote::{quote, ToTokens};
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
fn simple_for_loop() {
    let old = quote! {
        div {
            for item in vec![1, 2, 3] {
                div { "asasddasdasd" }
            }
        }
    };

    let new_valid = quote! {
        div {
            for item in vec![1, 2, 3] {
                div { "asasddasdasd" }
                div { "123" }
            }
        }
    };

    let new_invalid = quote! {
        div {
            for item in vec![1, 2, 3, 4] {
                div { "asasddasdasd" }
                div { "123" }
            }
        }
    };

    let location = "testing";
    let old: CallBody = syn::parse2(old).unwrap();
    let new_valid: CallBody = syn::parse2(new_valid).unwrap();
    let new_invalid: CallBody = syn::parse2(new_invalid).unwrap();

    assert!(hotreload_callbody::<Mock>(&old, &new_valid, location).is_some());
    assert!(hotreload_callbody::<Mock>(&old, &new_invalid, location).is_none());
}

#[test]
fn multiple_for_loops() {
    let old = quote! {
        div {
            for item in vec![1, 2, 3] {
                div { "asasddasdasd" }
            }
            for item in vec![4, 5, 6] {
                div { "asasddasdasd" }
            }
        }
    };

    // do a little reorder, still valid just different
    let new_valid = quote! {
        div {
            for item in vec![4, 5, 6] {
                span { "asasddasdasd" }
                span { "123" }
            }
            for item in vec![1, 2, 3] {
                div { "asasddasdasd" }
                div { "123" }
            }
        }
    };

    let new_invalid = quote! {
        div {
            for item in vec![1, 2, 3, 4] {
                div { "asasddasdasd" }
                div { "123" }
            }
            for item in vec![4, 5, 6] {
                span { "asasddasdasd" }
                span { "123" }
            }
        }
    };

    // just remove an entire for loop
    let new_valid_removed = quote! {
        div {
            for item in vec![4, 5, 6] {
                span { "asasddasdasd" }
                span { "123" }
            }
        }
    };

    let new_invalid_new_dynamic_internal = quote! {
        div {
            for item in vec![1, 2, 3] {
                div { "asasddasdasd" }
                div { "123" }
            }
            for item in vec![4, 5, 6] {
                span { "asasddasdasd" }

                // this is a new dynamic node, and thus can't be hot reloaded
                // Eventualy we might be able to do a format like this, but not right now
                span { "123 {item}" }
            }
        }
    };

    let new_invlaid_added = quote! {
        div {
            for item in vec![1, 2, 3] {
                div { "asasddasdasd" }
                div { "123" }
            }
            for item in vec![4, 5, 6] {
                span { "asasddasdasd" }
                span { "123" }
            }

            for item in vec![7, 8, 9] {
                span { "asasddasdasd" }
                span { "123" }
            }
        }
    };

    let location = "testing";
    let old: CallBody = syn::parse2(old).unwrap();
    let new_valid: CallBody = syn::parse2(new_valid).unwrap();
    let new_invalid: CallBody = syn::parse2(new_invalid).unwrap();
    let new_valid_removed: CallBody = syn::parse2(new_valid_removed).unwrap();
    let new_invalid_new_dynamic_internal: CallBody =
        syn::parse2(new_invalid_new_dynamic_internal).unwrap();
    let new_invlaid_added: CallBody = syn::parse2(new_invlaid_added).unwrap();

    let valid = hotreload_callbody::<Mock>(&old, &new_valid, location);
    assert!(valid.is_some());
    let templates = valid.unwrap();
    assert_eq!(templates.len(), 1);
    let template = &templates[0];
    // It's an inversion, so we should get them in reverse
    assert_eq!(template.node_paths, &[&[0, 1], &[0, 0]]);

    assert!(hotreload_callbody::<Mock>(&old, &new_invalid, location).is_none());
    assert!(
        hotreload_callbody::<Mock>(&old, &new_invalid_new_dynamic_internal, location).is_none()
    );

    let removed = hotreload_callbody::<Mock>(&old, &new_valid_removed, location);
    assert!(removed.is_some());
    let templates = removed.unwrap();
    assert_eq!(templates.len(), 1);
    let template = &templates[0];

    // We just completely removed the dynamic node, so it should be a "dud" path and then the placement
    assert_eq!(template.node_paths, &[&[], &[0u8, 0] as &[u8]]);

    // Adding a new dynamic node should not be hot reloadable
    let added = hotreload_callbody::<Mock>(&old, &new_invlaid_added, location);
    assert!(added.is_none());
}
