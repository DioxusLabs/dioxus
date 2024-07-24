#![allow(unused)]

use dioxus_core::{prelude::Template, VNode};
use dioxus_rsx::{
    hot_reload::{diff_rsx, ChangedRsx},
    hotreload::HotReloadedTemplate,
    CallBody, HotReloadingContext,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse::Parse, spanned::Spanned, token::Token, File};

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

fn boilerplate(old: TokenStream, new: TokenStream) -> Option<Vec<Template>> {
    let old: CallBody = syn::parse2(old).unwrap();
    let new: CallBody = syn::parse2(new).unwrap();

    let location = "file:line:col:0";
    hotreload_callbody::<Mock>(&old, &new, location)
}

fn can_hotreload(old: TokenStream, new: TokenStream) -> Option<HotReloadedTemplate> {
    let old: CallBody = syn::parse2(old).unwrap();
    let new: CallBody = syn::parse2(new).unwrap();

    let location = "file:line:col:0";
    let results = HotReloadedTemplate::new::<Mock>(&old, &new, location, Default::default())?;
    Some(results)
}

fn hotreload_callbody<Ctx: HotReloadingContext>(
    old: &CallBody,
    new: &CallBody,
    location: &'static str,
) -> Option<Vec<Template>> {
    let results = HotReloadedTemplate::new::<Ctx>(old, new, location, Default::default())?;
    Some(results.templates)
}

fn callbody_to_template<Ctx: HotReloadingContext>(
    old: &CallBody,
    location: &'static str,
) -> Option<Template> {
    let results = HotReloadedTemplate::new::<Ctx>(old, old, location, Default::default())?;
    Some(*results.templates.first().unwrap())
}

fn base_stream() -> TokenStream {
    quote! {
        div {
            for item in vec![1, 2, 3] {
                div { "asasddasdasd" }
            }
            for item in vec![4, 5, 6] {
                div { "asasddasdasd" }
            }
        }
    }
}

fn base() -> CallBody {
    syn::parse2(base_stream()).unwrap()
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

    let location = "file:line:col:0";
    let old: CallBody = syn::parse2(old).unwrap();
    let new_valid: CallBody = syn::parse2(new_valid).unwrap();
    let new_invalid: CallBody = syn::parse2(new_invalid).unwrap();

    assert!(hotreload_callbody::<Mock>(&old, &new_valid, location).is_some());
    assert!(hotreload_callbody::<Mock>(&old, &new_invalid, location).is_none());
}

#[test]
fn valid_reorder() {
    let old = base();
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

    let location = "file:line:col:0";
    let new: CallBody = syn::parse2(new_valid).unwrap();

    let valid = hotreload_callbody::<Mock>(&old, &new, location);
    assert!(valid.is_some());
    let templates = valid.unwrap();

    // Currently we return all the templates, even if they didn't change
    assert_eq!(templates.len(), 3);

    let template = &templates[2];

    // It's an inversion, so we should get them in reverse
    assert_eq!(template.node_paths, &[&[0, 1], &[0, 0]]);

    // And the byte index should be the original template
    assert_eq!(template.name, "file:line:col:0");
}

#[test]
fn invalid_cases() {
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
                // Eventually we might be able to do a format like this, but not right now
                span { "123 {item}" }
            }
        }
    };

    let new_invalid_added = quote! {
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

    let location = "file:line:col:0";
    let old = base();

    let new_invalid: CallBody = syn::parse2(new_invalid).unwrap();
    let new_valid_removed: CallBody = syn::parse2(new_valid_removed).unwrap();
    let new_invalid_new_dynamic_internal: CallBody =
        syn::parse2(new_invalid_new_dynamic_internal).unwrap();
    let new_invalid_added: CallBody = syn::parse2(new_invalid_added).unwrap();

    assert!(hotreload_callbody::<Mock>(&old, &new_invalid, location).is_none());
    assert!(
        hotreload_callbody::<Mock>(&old, &new_invalid_new_dynamic_internal, location).is_none()
    );

    let removed = hotreload_callbody::<Mock>(&old, &new_valid_removed, location);
    assert!(removed.is_some());
    let templates = removed.unwrap();

    // we don't get the removed template back
    assert_eq!(templates.len(), 2);
    let template = &templates[1];

    // We just completely removed the dynamic node, so it should be a "dud" path and then the placement
    assert_eq!(template.node_paths, &[&[], &[0u8, 0] as &[u8]]);

    // Adding a new dynamic node should not be hot reloadable
    let added = hotreload_callbody::<Mock>(&old, &new_invalid_added, location);
    assert!(added.is_none());
}

#[test]
fn new_names() {
    let old = quote! {
        div {
            for item in vec![1, 2, 3] {
                div { "asasddasdasd" }
                div { "123" }
            }
        }
    };

    // Same order, just different contents
    let new_valid_internal = quote! {
        div {
            for item in vec![1, 2, 3] {
                div { "asasddasdasd" }
                div { "456" }
            }
        }
    };

    let templates = boilerplate(old, new_valid_internal).unwrap();

    // Getting back all the templates even though some might not have changed
    // This is currently just a symptom of us not checking if anything has changed, but has no bearing
    // on output really.
    assert_eq!(templates.len(), 2);

    // The ordering is going to be inverse since its a depth-first traversal
    let external = &templates[1];
    assert_eq!(external.name, "file:line:col:0");

    let internal = &templates[0];
    assert_eq!(internal.name, "file:line:col:1");
}

#[test]
fn attributes_reload() {
    let old = quote! {
        div {
            class: "{class}",
            id: "{id}",
            name: "name",
        }
    };

    // Same order, just different contents
    let new_valid_internal = quote! {
        div {
            id: "{id}",
            name: "name",
            class: "{class}"
        }
    };

    let templates = boilerplate(old, new_valid_internal).unwrap();

    dbg!(templates);
}

#[test]
fn template_generates() {
    let old = quote! {
        svg {
            width: 100,
            height: "100px",
            "width2": 100,
            "height2": "100px",
            p { "hello world" }
            {(0..10).map(|i| rsx!{"{i}"})}
        }
        div {
            width: 120,
            div {
                height: "100px",
                "width2": 130,
                "height2": "100px",
                for i in 0..10 {
                    div {
                        "asdasd"
                    }
                }
            }
        }
    };

    let old: CallBody = syn::parse2(old).unwrap();
    let template = callbody_to_template::<Mock>(&old, "file:line:col:0");
}

#[test]
fn diffs_complex() {
    #[allow(unused, non_snake_case)]
    fn Comp() -> dioxus_core::Element {
        VNode::empty()
    }

    let old = quote! {
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

    // scrambling the attributes should not cause a full rebuild
    let new = quote! {
        div {
            width: 100,
            height: "100px",
            "width2": 100,
            "height2": "100px",
            p { "hello world" }
            Comp {}
            {(0..10).map(|i| rsx!{"{i}"})},
            {(0..10).map(|i| rsx!{"{i}"})},
            {(0..11).map(|i| rsx!{"{i}"})},
        }
    };

    let old: CallBody = syn::parse2(old).unwrap();
    let new: CallBody = syn::parse2(new).unwrap();

    let location = "file:line:col:0";
    let templates = hotreload_callbody::<Mock>(&old, &new, location).unwrap();
}

#[test]
fn remove_node() {
    let changed = boilerplate(
        quote! {
            svg {
                Comp {}
                {(0..10).map(|i| rsx!{"{i}"})},
            }
        },
        quote! {
            div {
                {(0..10).map(|i| rsx!{"{i}"})},
            }
        },
    )
    .unwrap();

    dbg!(changed);
}

#[test]
fn if_chains() {
    let changed = boilerplate(
        quote! {
            if cond {
                "foo"
            }
        },
        quote! {
            if cond {
                "baz"
            }
        },
    )
    .unwrap();

    let very_complex_chain = boilerplate(
        quote! {
            if cond {
                if second_cond {
                    "foo"
                }
            } else if othercond {
                "bar"
            } else {
                "baz"
            }
        },
        quote! {
            if cond {
                if second_cond {
                    span { "asasddasdasd 789" }
                }
            } else if othercond {
                span { "asasddasdasd 123" }
            } else {
                span { "asasddasdas 456" }
            }
        },
    )
    .unwrap();

    dbg!(very_complex_chain);
}

#[test]
fn component_bodies() {
    let changed = boilerplate(
        quote! {
            Comp {
                "foo"
            }
        },
        quote! {
            Comp {
                "baz"
            }
        },
    )
    .unwrap();

    dbg!(changed);
}

/// Everything reloads!
#[test]
fn kitch_sink_of_reloadability() {
    let changed = boilerplate(
        quote! {
            div {
                for i in 0..10 {
                    div { "123" }
                    Comp {
                        "foo"
                    }
                    if cond {
                        "foo"
                    }
                }
            }
        },
        quote! {
            div {
                "hi!"
                for i in 0..10 {
                    div { "456" }
                    Comp { "bar" }
                    if cond {
                        "baz"
                    }
                }
            }
        },
    )
    .unwrap();

    dbg!(changed);
}

/// Moving nodes inbetween multiple rsx! calls currently doesn't work
/// Sad. Needs changes to core to work, and is technically flawed?
#[test]
fn entire_kitchen_sink() {
    let changed = boilerplate(
        quote! {
            div {
                for i in 0..10 {
                    div { "123" }
                }
                Comp {
                    "foo"
                }
                if cond {
                    "foo"
                }
            }
        },
        quote! {
            div {
                "hi!"
                Comp {
                    for i in 0..10 {
                        div { "456" }
                    }
                    "bar"
                    if cond {
                        "baz"
                    }
                }
            }
        },
    );

    assert!(changed.is_none());
}

#[test]
fn tokenstreams_and_locations() {
    let changed = boilerplate(
        quote! {
            div { "hhi" }
            div {
                {rsx! { "hi again!" }},
                for i in 0..2 {
                    "first"
                    div { "hi {i}" }
                }

                for i in 0..3 {
                    "Second"
                    div { "hi {i}" }
                }

                if false {
                    div { "hi again!?" }
                } else if true {
                    div { "its cool?" }
                } else {
                    div { "not nice !" }
                }
            }
        },
        quote! {
            div { "hhi" }
            div {
                {rsx! { "hi again!" }},
                for i in 0..2 {
                    "first"
                    div { "hi {i}" }
                }

                for i in 0..3 {
                    "Second"
                    div { "hi {i}" }
                }

                if false {
                    div { "hi again?" }
                } else if true {
                    div { "cool?" }
                } else {
                    div { "nice !" }
                }
            }

        },
    );

    dbg!(changed);
}

#[test]
fn ide_testcase() {
    let changed = boilerplate(
        quote! {
            div {
                div { "hi!!!123 in!stant relo123a1123dasasdasdasdasd" }
                for x in 0..5 {
                    h3 { "For loop contents" }
                }
            }
        },
        quote! {
            div {
                div { "hi!!!123 in!stant relo123a1123dasasdasdasdasd" }
                for x in 0..5 {
                    h3 { "For loop contents" }
                }
            }
        },
    );

    dbg!(changed);
}

#[test]
fn assigns_ids() {
    let toks = quote! {
        div {
            div { "hi!!!123 in!stant relo123a1123dasasdasdasdasd" }
            for x in 0..5 {
                h3 { "For loop contents" }
            }
        }
    };

    let parsed = syn::parse2::<CallBody>(toks).unwrap();

    let node = parsed.body.get_dyn_node(&[0, 1]);
    dbg!(node);
}

#[test]
fn simple_start() {
    let changed = boilerplate(
        //
        quote! {
            div {
                class: "Some {one}",
                id: "Something {two}",
                "One"
            }
        },
        quote! {
            div {
                id: "Something {two}",
                class: "Some {one}",
                "One"
            }
        },
    );

    dbg!(changed.unwrap());
}

#[test]
fn complex_cases() {
    let changed = can_hotreload(
        quote! {
            div {
                class: "Some {one}",
                id: "Something {two}",
                "One"
            }
        },
        quote! {
            div {
                class: "Some {one}",
                id: "Something else {two}",
                "One"
            }
        },
    );

    dbg!(changed.unwrap());
}

#[test]
fn attribute_cases() {
    let changed = can_hotreload(
        quote! {
            div {
                class: "Some {one}",
                id: "Something {two}",
                "One"
            }
        },
        quote! {
            div {
                id: "Something {two}",
                "One"
            }
        },
    );
    dbg!(changed.unwrap());

    let changed = can_hotreload(
        //
        quote! { div { class: 123 } },
        quote! { div { class: 456 } },
    );
    dbg!(changed.unwrap());

    let changed = can_hotreload(
        //
        quote! { div { class: 123.0 } },
        quote! { div { class: 456.0 } },
    );
    dbg!(changed.unwrap());

    let changed = can_hotreload(
        //
        quote! { div { class: "asd {123}", } },
        quote! { div { class: "def", } },
    );
    dbg!(changed.unwrap());
}

#[test]
fn text_node_cases() {
    let changed = can_hotreload(
        //
        quote! { div { "hello {world}" } },
        quote! { div { "world {world}" } },
    );
    dbg!(changed.unwrap());

    let changed = can_hotreload(
        //
        quote! { div { "hello {world}" } },
        quote! { div { "world" } },
    );
    dbg!(changed.unwrap());

    let changed = can_hotreload(
        //
        quote! { div { "hello" } },
        quote! { div { "world {world}" } },
    );
    assert!(changed.is_none());
}

#[test]
fn simple_carry() {
    let a = quote! {
        // start with
        "thing {abc} {def}"       // 1, 1, 1
        "thing {def}"             // 1, 0, 1
        "other {hij}" // 1, 1, 1
    };

    let b = quote! {
        // end with
        "thing {def}"
        "thing {abc}"
        "thing {hij}"
    };

    let changed = can_hotreload(a, b);
    dbg!(changed.unwrap());
}

#[test]
fn complex_carry_text() {
    let a = quote! {
        // start with
        "thing {abc} {def}"       // 1, 1, 1
        "thing {abc}"             // 1, 0, 1
        "other {abc} {def} {hij}" // 1, 1, 1
    };

    let b = quote! {
        // end with
        "thing {abc}"
        "thing {hij}"
    };

    let changed = can_hotreload(a, b);
    dbg!(changed.unwrap());
}

#[test]
fn complex_carry() {
    let a = quote! {
        Component {
            class: "thing {abc}",
            other: "other {abc} {def}",
        }
        Component {
            class: "thing {abc}",
            other: "other",
        }
    };

    let b = quote! {
        // how about shuffling components, for, if, etc
        Component {
            class: "thing {abc}",
            other: "other {abc} {def}",
        }
        Component {
            class: "thing",
            other: "other",
        }
    };

    let changed = can_hotreload(a, b);
    dbg!(changed.unwrap());
}

#[test]
fn component_with_lits() {
    let a = quote! {
        Component {
            class: 123,
            id: 456.789,
            other: true,
            blah: "hello {world}",
        }
    };

    // changing lit values
    let b = quote! {
        Component {
            class: 456,
            id: 789.456,
            other: false,
            blah: "goodbye {world}",
        }
    };

    let changed = can_hotreload(a, b);
    dbg!(changed.unwrap());
}

#[test]
fn component_with_handlers() {
    let a = quote! {
        Component {
            class: 123,
            id: 456.789,
            other: true,
            blah: "hello {world}",
            onclick: |e| { println!("clicked") },
        }
    };

    // changing lit values
    let b = quote! {
        Component {
            class: 456,
            id: 789.456,
            other: false,
            blah: "goodbye {world}",
            onclick: |e| { println!("clicked") },
        }
    };

    let changed = can_hotreload(a, b);
    dbg!(changed.unwrap());
}
