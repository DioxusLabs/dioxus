use dioxus_rsx::{CallBody, HotReloadingContext};
use quote::quote;

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
fn create_template() {
    let input = quote! {
        svg {
            width: 100,
            height: "100px",
            "width2": 100,
            "height2": "100px",
            p { "hello world" }
            {(0..10).map(|i| rsx!{"{i}"})}
        }
    };

    let call_body: CallBody = syn::parse2(input).unwrap();
    let new_template = call_body.update_template::<Mock>(None, "testing").unwrap();
    insta::assert_debug_snapshot!(new_template);
}

#[test]
fn diff_template() {
    #[allow(unused, non_snake_case)]
    fn Comp() -> dioxus_core::Element {
        None
    }

    let input = quote! {
        svg {
            width: 100,
            height: "100px",
            "width2": 100,
            "height2": "100px",
            p { "hello world" }
            {(0..10).map(|i| rsx!{"{i}"})},
            {(0..10).map(|i| rsx!{"{i}"})},
            {(0..11).map(|i| rsx!{"{i}"})},
            Comp {}
        }
    };

    let call_body1: CallBody = syn::parse2(input).unwrap();
    let created_template = call_body1.update_template::<Mock>(None, "testing").unwrap();
    insta::assert_debug_snapshot!(created_template);

    // scrambling the attributes should not cause a full rebuild
    let input = quote! {
        div {
            "width2": 100,
            height: "100px",
            "height2": "100px",
            width: 100,
            Comp {}
            {(0..11).map(|i| rsx!{"{i}"})},
            {(0..10).map(|i| rsx!{"{i}"})},
            {(0..10).map(|i| rsx!{"{i}"})},
            p {
                "hello world"
            }
        }
    };

    let call_body2: CallBody = syn::parse2(input).unwrap();
    let new_template = call_body2
        .update_template::<Mock>(Some(call_body1), "testing")
        .unwrap();

    insta::assert_debug_snapshot!(new_template);
}

#[test]
fn changing_forloops_is_okay() {
    let input = quote! {
        div {
            for i in 0..10 {
                div { "123" }
                "asdasd"
            }
        }
    };

    let call_body: CallBody = syn::parse2(input).unwrap();
    let new_template = call_body.update_template::<Mock>(None, "testing").unwrap();

    dbg!(new_template);
}
