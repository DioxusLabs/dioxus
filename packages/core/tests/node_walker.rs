use crate::TemplateAttribute::Static;
use dioxus::prelude::*;
use dioxus_core::{prelude::NodeCursor, TemplateNode, Template};

#[test]
fn create_walker() {
    let node = rsx! {
        div {
            img {
                src: "",
            }
            div {
                p { "Hello World" }
            }
        }
    }
    .unwrap();

    let cursor = NodeCursor::new(node).first_child().unwrap();
    assert_eq!(
        cursor.current_node(),
        TemplateNode::Element {
            tag: "img",
            namespace: None,
            attrs: &[Static { name: "src", value: "", namespace: None }],
            children: &[]
        }
    );

    let cursor = cursor.next_sibling().unwrap();
    assert_eq!(
        cursor.current_node(),
        TemplateNode::Element {
            tag: "div",
            namespace: None,
            attrs: &[],
            children: &[TemplateNode::Element { tag: "p", namespace: None, attrs: &[], children: &[TemplateNode::Text { text: "Hello World" }] }]
        }
    );

    let cursor = cursor.first_child().unwrap();
    assert_eq!(
        cursor.current_node(),
        TemplateNode::Element { tag: "p", namespace: None, attrs: &[], children: &[TemplateNode::Text { text: "Hello World" }] } 
    );
}

#[test]
fn recursive_walker() {
    let node = rsx! {
        div {
            "Hello"
            p { "world" }
        }
    }
    .unwrap();

    let mut cursor = NodeCursor::new(node);

    for child in cursor.children() {
        if let Some(text) = child.as_text() {
            assert_eq!(text, "Hello")
        }
        for grandchild in child.children() {
            if let Some(text) = grandchild.as_text() {
                assert_eq!(text, "world")
            }
        }
    }
}
