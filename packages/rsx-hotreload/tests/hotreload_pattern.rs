#![allow(unused)]

use std::collections::HashMap;

use dioxus_core::{
    internal::{
        FmtSegment, FmtedSegments, HotReloadAttributeValue, HotReloadDynamicAttribute,
        HotReloadDynamicNode, HotReloadLiteral, HotReloadedTemplate, NamedAttribute,
    },
    prelude::{Template, TemplateNode},
    TemplateAttribute, VNode,
};
use dioxus_core_types::HotReloadingContext;
use dioxus_rsx::CallBody;
use dioxus_rsx_hotreload::{self, diff_rsx, ChangedRsx, HotReloadResult};
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

fn hot_reload_from_tokens(
    old: TokenStream,
    new: TokenStream,
) -> Option<HashMap<usize, HotReloadedTemplate>> {
    let old: CallBody = syn::parse2(old).unwrap();
    let new: CallBody = syn::parse2(new).unwrap();

    hotreload_callbody::<Mock>(&old, &new)
}

fn can_hotreload(old: TokenStream, new: TokenStream) -> bool {
    hot_reload_from_tokens(old, new).is_some()
}

fn hotreload_callbody<Ctx: HotReloadingContext>(
    old: &CallBody,
    new: &CallBody,
) -> Option<HashMap<usize, HotReloadedTemplate>> {
    let results = HotReloadResult::new::<Ctx>(&old.body, &new.body, Default::default())?;
    Some(results.templates)
}

fn callbody_to_template<Ctx: HotReloadingContext>(
    old: &CallBody,
    location: &'static str,
) -> Option<HotReloadedTemplate> {
    let mut results = HotReloadResult::new::<Ctx>(&old.body, &old.body, Default::default())?;
    Some(results.templates.remove(&0).unwrap())
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

    assert!(hotreload_callbody::<Mock>(&old, &new_valid).is_some());
    assert!(hotreload_callbody::<Mock>(&old, &new_invalid).is_none());
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

    let new: CallBody = syn::parse2(new_valid).unwrap();

    let valid = hotreload_callbody::<Mock>(&old, &new);
    assert!(valid.is_some());
    let templates = valid.unwrap();

    // Currently we return all the templates, even if they didn't change
    assert_eq!(templates.len(), 3);

    let template = &templates[&0];

    // It's an inversion, so we should get them in reverse
    assert_eq!(
        template.roots,
        &[TemplateNode::Element {
            tag: "div",
            namespace: None,
            attrs: &[],
            children: &[
                TemplateNode::Dynamic { id: 0 },
                TemplateNode::Dynamic { id: 1 }
            ]
        }]
    );
    assert_eq!(
        template.dynamic_nodes,
        &[
            HotReloadDynamicNode::Dynamic(1),
            HotReloadDynamicNode::Dynamic(0)
        ]
    );
}

#[test]
fn valid_new_node() {
    // Adding a new dynamic node should be hot reloadable as long as the text was present in the old version
    // of the rsx block
    let old = quote! {
        div {
            for item in vec![1, 2, 3] {
                div { "item is {item}" }
            }
        }
    };
    let new = quote! {
        div {
            for item in vec![1, 2, 3] {
                div { "item is {item}" }
                div { "item is also {item}" }
            }
        }
    };

    let templates = hot_reload_from_tokens(old, new).unwrap();

    // Currently we return all the templates, even if they didn't change
    assert_eq!(templates.len(), 2);

    let template = &templates[&1];

    // The new dynamic node should be created from the formatted segments pool
    assert_eq!(
        template.dynamic_nodes,
        &[
            HotReloadDynamicNode::Formatted(FmtedSegments::new(vec![
                FmtSegment::Literal { value: "item is " },
                FmtSegment::Dynamic { id: 0 }
            ],)),
            HotReloadDynamicNode::Formatted(FmtedSegments::new(vec![
                FmtSegment::Literal {
                    value: "item is also "
                },
                FmtSegment::Dynamic { id: 0 }
            ],)),
        ]
    );
}

#[test]
fn valid_new_dynamic_attribute() {
    // Adding a new dynamic attribute should be hot reloadable as long as the text was present in the old version
    // of the rsx block
    let old = quote! {
        div {
            for item in vec![1, 2, 3] {
                div {
                    class: "item is {item}"
                }
            }
        }
    };
    let new = quote! {
        div {
            for item in vec![1, 2, 3] {
                div {
                    class: "item is {item}"
                }
                div {
                    class: "item is also {item}"
                }
            }
        }
    };

    let templates = hot_reload_from_tokens(old, new).unwrap();

    // Currently we return all the templates, even if they didn't change
    assert_eq!(templates.len(), 2);

    let template = &templates[&1];

    // We should have a new dynamic attribute
    assert_eq!(
        template.roots,
        &[
            TemplateNode::Element {
                tag: "div",
                namespace: None,
                attrs: &[TemplateAttribute::Dynamic { id: 0 }],
                children: &[]
            },
            TemplateNode::Element {
                tag: "div",
                namespace: None,
                attrs: &[TemplateAttribute::Dynamic { id: 1 }],
                children: &[]
            }
        ]
    );

    // The new dynamic attribute should be created from the formatted segments pool
    assert_eq!(
        template.dynamic_attributes,
        &[
            HotReloadDynamicAttribute::Named(NamedAttribute::new(
                "class",
                None,
                HotReloadAttributeValue::Literal(HotReloadLiteral::Fmted(FmtedSegments::new(
                    vec![
                        FmtSegment::Literal { value: "item is " },
                        FmtSegment::Dynamic { id: 0 }
                    ],
                )))
            )),
            HotReloadDynamicAttribute::Named(NamedAttribute::new(
                "class",
                None,
                HotReloadAttributeValue::Literal(HotReloadLiteral::Fmted(FmtedSegments::new(
                    vec![
                        FmtSegment::Literal {
                            value: "item is also "
                        },
                        FmtSegment::Dynamic { id: 0 }
                    ],
                )))
            )),
        ]
    );
}

#[test]
fn valid_move_dynamic_segment_between_nodes() {
    // Hot reloading should let you move around a dynamic formatted segment between nodes
    let old = quote! {
        div {
            for item in vec![1, 2, 3] {
                div {
                    class: "item is {item}"
                }
            }
        }
    };
    let new = quote! {
        div {
            for item in vec![1, 2, 3] {
                "item is {item}"
            }
        }
    };

    let templates = hot_reload_from_tokens(old, new).unwrap();

    // Currently we return all the templates, even if they didn't change
    assert_eq!(templates.len(), 2);

    let template = &templates[&1];

    // We should have a new dynamic node and no attributes
    assert_eq!(template.roots, &[TemplateNode::Dynamic { id: 0 }]);

    // The new dynamic node should be created from the formatted segments pool
    assert_eq!(
        template.dynamic_nodes,
        &[HotReloadDynamicNode::Formatted(FmtedSegments::new(vec![
            FmtSegment::Literal { value: "item is " },
            FmtSegment::Dynamic { id: 0 }
        ])),]
    );
}

#[test]
fn valid_keys() {
    let a = quote! {
        div {
            key: "{value}",
        }
    };

    // we can clone dynamic nodes to hot reload them
    let b = quote! {
        div {
            key: "{value}-1234",
        }
    };

    let hot_reload = hot_reload_from_tokens(a, b).unwrap();

    assert_eq!(hot_reload.len(), 1);

    let template = &hot_reload[&0];

    assert_eq!(
        template.key,
        Some(FmtedSegments::new(vec![
            FmtSegment::Dynamic { id: 0 },
            FmtSegment::Literal { value: "-1234" }
        ]))
    );
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

    assert!(hotreload_callbody::<Mock>(&old, &new_invalid).is_none());
    assert!(hotreload_callbody::<Mock>(&old, &new_invalid_new_dynamic_internal).is_none());

    let templates = hotreload_callbody::<Mock>(&old, &new_valid_removed).unwrap();

    // we don't get the removed template back
    assert_eq!(templates.len(), 2);
    let template = &templates.get(&0).unwrap();

    // We just completely removed the dynamic node, so it should be a "dud" path and then the placement
    assert_eq!(
        template.roots,
        &[TemplateNode::Element {
            tag: "div",
            namespace: None,
            attrs: &[],
            children: &[TemplateNode::Dynamic { id: 0 }]
        }]
    );
    assert_eq!(template.dynamic_nodes, &[HotReloadDynamicNode::Dynamic(1)]);

    // Adding a new dynamic node should not be hot reloadable
    let added = hotreload_callbody::<Mock>(&old, &new_invalid_added);
    assert!(added.is_none());
}

#[test]
fn invalid_empty_rsx() {
    let old_template = quote! {
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

    // empty out the whole rsx block
    let new_template = quote! {};

    let location = "file:line:col:0";

    let old_template: CallBody = syn::parse2(old_template).unwrap();
    let new_template: CallBody = syn::parse2(new_template).unwrap();

    assert!(hotreload_callbody::<Mock>(&old_template, &new_template).is_none());
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

    let templates = hot_reload_from_tokens(old, new_valid_internal).unwrap();

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
            {(0..10).map(|i| rsx! {"{i}"})}
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
            {(0..10).map(|i| rsx! {"{i}"})},
            {(0..10).map(|i| rsx! {"{i}"})},
            {(0..11).map(|i| rsx! {"{i}"})},
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
            {(0..10).map(|i| rsx! {"{i}"})},
            {(0..10).map(|i| rsx! {"{i}"})},
            {(0..11).map(|i| rsx! {"{i}"})},
        }
    };

    let old: CallBody = syn::parse2(old).unwrap();
    let new: CallBody = syn::parse2(new).unwrap();

    let templates = hotreload_callbody::<Mock>(&old, &new).unwrap();
}

#[test]
fn remove_node() {
    let valid = hot_reload_from_tokens(
        quote! {
            svg {
                Comp {}
                {(0..10).map(|i| rsx! {"{i}"})},
            }
        },
        quote! {
            div {
                {(0..10).map(|i| rsx! {"{i}"})},
            }
        },
    )
    .unwrap();

    dbg!(valid);
}

#[test]
fn if_chains() {
    let valid = hot_reload_from_tokens(
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

    let very_complex_chain = hot_reload_from_tokens(
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
    let valid = can_hotreload(
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
    );

    assert!(valid);
}

// We currently don't track aliasing which means we can't allow dynamic nodes/formatted segments to be moved between scopes
#[test]
fn moving_between_scopes() {
    let valid = can_hotreload(
        quote! {
            for x in 0..10 {
                for y in 0..10 {
                    div { "x is {x}" }
                }
            }
        },
        quote! {
            for x in 0..10 {
                div { "x is {x}" }
            }
        },
    );

    assert!(!valid);
}

/// Everything reloads!
#[test]
fn kitch_sink_of_reloadability() {
    let valid = hot_reload_from_tokens(
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

    dbg!(valid);
}

/// Moving nodes inbetween multiple rsx! calls currently doesn't work
/// Sad. Needs changes to core to work, and is technically flawed?
#[test]
fn entire_kitchen_sink() {
    let valid = hot_reload_from_tokens(
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

    assert!(valid.is_none());
}

#[test]
fn tokenstreams_and_locations() {
    let valid = hot_reload_from_tokens(
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

    dbg!(valid);
}

#[test]
fn ide_testcase() {
    let valid = hot_reload_from_tokens(
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

    dbg!(valid);
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
    let valid = can_hotreload(
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

    assert!(valid);
}

#[test]
fn complex_cases() {
    let valid = can_hotreload(
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

    assert!(valid);
}

#[test]
fn attribute_cases() {
    let valid = can_hotreload(
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
    assert!(valid);

    let valid = can_hotreload(
        //
        quote! { div { class: 123 } },
        quote! { div { class: 456 } },
    );
    assert!(valid);

    let valid = can_hotreload(
        //
        quote! { div { class: 123.0 } },
        quote! { div { class: 456.0 } },
    );
    assert!(valid);

    let valid = can_hotreload(
        //
        quote! { div { class: "asd {123}", } },
        quote! { div { class: "def", } },
    );
    assert!(valid);
}

#[test]
fn text_node_cases() {
    let valid = can_hotreload(
        //
        quote! { div { "hello {world}" } },
        quote! { div { "world {world}" } },
    );
    assert!(valid);

    let valid = can_hotreload(
        //
        quote! { div { "hello {world}" } },
        quote! { div { "world" } },
    );
    assert!(valid);

    let valid = can_hotreload(
        //
        quote! { div { "hello {world}" } },
        quote! { div { "world {world} {world}" } },
    );
    assert!(valid);

    let valid = can_hotreload(
        //
        quote! { div { "hello" } },
        quote! { div { "world {world}" } },
    );
    assert!(!valid);
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

    let valid = can_hotreload(a, b);
    assert!(valid);
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

    let valid = can_hotreload(a, b);
    assert!(valid);
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

    let valid = can_hotreload(a, b);
    assert!(valid);
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

    let valid = can_hotreload(a, b);
    assert!(valid);
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

    let hot_reload = hot_reload_from_tokens(a, b).unwrap();
    let template = hot_reload.get(&0).unwrap();
    assert_eq!(
        template.component_values,
        &[
            HotReloadLiteral::Int(456),
            HotReloadLiteral::Float(789.456),
            HotReloadLiteral::Bool(false),
            HotReloadLiteral::Fmted(FmtedSegments::new(vec![
                FmtSegment::Literal { value: "goodbye " },
                FmtSegment::Dynamic { id: 0 }
            ])),
        ]
    );
}

#[test]
fn component_remove_key() {
    let a = quote! {
        Component {
            key: "{key}",
            class: 123,
            id: 456.789,
            other: true,
            dynamic1,
            dynamic2,
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
            dynamic1,
            dynamic2,
            blah: "goodbye {world}",
            onclick: |e| { println!("clicked") },
        }
    };

    let hot_reload = hot_reload_from_tokens(a, b).unwrap();
    let template = hot_reload.get(&0).unwrap();
    assert_eq!(
        template.component_values,
        &[
            HotReloadLiteral::Int(456),
            HotReloadLiteral::Float(789.456),
            HotReloadLiteral::Bool(false),
            HotReloadLiteral::Fmted(FmtedSegments::new(vec![
                FmtSegment::Literal { value: "goodbye " },
                FmtSegment::Dynamic { id: 1 }
            ]))
        ]
    );
}

#[test]
fn component_modify_key() {
    let a = quote! {
        Component {
            key: "{key}",
            class: 123,
            id: 456.789,
            other: true,
            dynamic1,
            dynamic2,
            blah1: "hello {world123}",
            blah2: "hello {world}",
            onclick: |e| { println!("clicked") },
        }
    };

    // changing lit values
    let b = quote! {
        Component {
            key: "{key}-{world}",
            class: 456,
            id: 789.456,
            other: false,
            dynamic1,
            dynamic2,
            blah1: "hello {world123}",
            blah2: "hello {world}",
            onclick: |e| { println!("clicked") },
        }
    };

    let hot_reload = hot_reload_from_tokens(a, b).unwrap();
    let template = hot_reload.get(&0).unwrap();
    assert_eq!(
        template.key,
        Some(FmtedSegments::new(vec![
            FmtSegment::Dynamic { id: 0 },
            FmtSegment::Literal { value: "-" },
            FmtSegment::Dynamic { id: 2 },
        ]))
    );
    assert_eq!(
        template.component_values,
        &[
            HotReloadLiteral::Int(456),
            HotReloadLiteral::Float(789.456),
            HotReloadLiteral::Bool(false),
            HotReloadLiteral::Fmted(FmtedSegments::new(vec![
                FmtSegment::Literal { value: "hello " },
                FmtSegment::Dynamic { id: 1 }
            ])),
            HotReloadLiteral::Fmted(FmtedSegments::new(vec![
                FmtSegment::Literal { value: "hello " },
                FmtSegment::Dynamic { id: 2 }
            ]))
        ]
    );
}

#[test]
fn duplicating_dynamic_nodes() {
    let a = quote! {
        div {
            {some_expr}
        }
    };

    // we can clone dynamic nodes to hot reload them
    let b = quote! {
        div {
            {some_expr}
            {some_expr}
        }
    };

    let valid = can_hotreload(a, b);
    assert!(valid);
}

#[test]
fn duplicating_dynamic_attributes() {
    let a = quote! {
        div {
            width: value,
        }
    };

    // we can clone dynamic nodes to hot reload them
    let b = quote! {
        div {
            width: value,
            height: value,
        }
    };

    let valid = can_hotreload(a, b);
    assert!(valid);
}

// We should be able to fill in empty nodes
#[test]
fn valid_fill_empty() {
    let valid = can_hotreload(
        quote! {},
        quote! {
            div { "x is 123" }
        },
    );

    assert!(valid);
}

// We should be able to hot reload spreads
#[test]
fn valid_spread() {
    let valid = can_hotreload(
        quote! {
            div {
                ..spread
            }
        },
        quote! {
            div {
                "hello world"
            }
            h1 {
                ..spread
            }
        },
    );

    assert!(valid);
}
