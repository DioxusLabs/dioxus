use dioxus_core::{Template, TemplateAttribute, TemplateNode};
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

    let template = call_body.update_template::<Mock>(None, "testing").unwrap();

    dbg!(template);

    assert_eq!(
        template,
        Template {
            name: "testing",
            roots: &[TemplateNode::Element {
                tag: "svg",
                namespace: Some("svg"),
                attrs: &[
                    TemplateAttribute::Dynamic { id: 0 },
                    TemplateAttribute::Static {
                        name: "height",
                        namespace: Some("style"),
                        value: "100px",
                    },
                    TemplateAttribute::Dynamic { id: 1 },
                    TemplateAttribute::Static {
                        name: "height2",
                        namespace: None,
                        value: "100px",
                    },
                ],
                children: &[
                    TemplateNode::Element {
                        tag: "p",
                        namespace: None,
                        attrs: &[],
                        children: &[TemplateNode::Text {
                            text: "hello world",
                        }],
                    },
                    TemplateNode::Dynamic { id: 0 }
                ],
            }],
            node_paths: &[&[0, 1,],],
            attr_paths: &[&[0,], &[0,],],
        },
    )
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

    let template = call_body1.update_template::<Mock>(None, "testing").unwrap();
    dbg!(template);

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

    let template = call_body2
        .update_template::<Mock>(Some(call_body1), "testing")
        .unwrap();

    dbg!(template);

    assert_eq!(
        template,
        Template {
            name: "testing",
            roots: &[TemplateNode::Element {
                tag: "div",
                namespace: None,
                attrs: &[
                    TemplateAttribute::Dynamic { id: 1 },
                    TemplateAttribute::Static {
                        name: "height",
                        namespace: None,
                        value: "100px",
                    },
                    TemplateAttribute::Static {
                        name: "height2",
                        namespace: None,
                        value: "100px",
                    },
                    TemplateAttribute::Dynamic { id: 0 },
                ],
                children: &[
                    TemplateNode::Dynamic { id: 3 },
                    TemplateNode::Dynamic { id: 2 },
                    TemplateNode::Dynamic { id: 1 },
                    TemplateNode::Dynamic { id: 0 },
                    TemplateNode::Element {
                        tag: "p",
                        namespace: None,
                        attrs: &[],
                        children: &[TemplateNode::Text {
                            text: "hello world",
                        }],
                    },
                ],
            }],
            node_paths: &[&[0, 3], &[0, 2], &[0, 1], &[0, 0]],
            attr_paths: &[&[0], &[0]]
        },
    )
}
