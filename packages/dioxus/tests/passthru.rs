#![allow(unused, non_upper_case_globals)]
#![allow(non_snake_case)]

//! Diffing Tests
//!
//! These tests only verify that the diffing algorithm works properly for single components.
//!
//! It does not validated that component lifecycles work properly. This is done in another test file.

use dioxus::prelude::*;

fn new_dom() -> VirtualDom {
    VirtualDom::new(|cx| render!("hi"))
}

use dioxus_core::DomEdit::*;

/// Should push the text node onto the stack and modify it
#[test]
fn nested_passthru_creates() {
    fn app(cx: Scope) -> Element {
        cx.render(rsx! {
            Child {
                Child {
                    Child {
                        div {
                            "hi"
                        }
                    }
                }
            }
        })
    };

    #[inline_props]
    fn Child<'a>(cx: Scope, children: Element<'a>) -> Element {
        cx.render(rsx! { children })
    };

    let mut dom = VirtualDom::new(app);
    let mut channel = dom.get_scheduler_channel();
    assert!(dom.has_work());

    let edits = dom.rebuild();
    assert_eq!(
        edits.edits,
        [
            CreateElement { tag: "div", root: Some(1), children: 0 },
            CreateTextNode { text: "hi", root: Some(2) },
            AppendChildren { root: Some(1), children: vec![2] },
            AppendChildren { root: Some(0), children: vec![1] },
        ]
    )
}

/// Should push the text node onto the stack and modify it
#[test]
fn nested_passthru_creates_add() {
    fn app(cx: Scope) -> Element {
        cx.render(rsx! {
            Child {
                "1"
                Child {
                    "2"
                    Child {
                        "3"
                        div {
                            "hi"
                        }
                    }
                }
            }
        })
    };

    #[inline_props]
    fn Child<'a>(cx: Scope, children: Element<'a>) -> Element {
        cx.render(rsx! {
                children
        })
    };

    let mut dom = VirtualDom::new(app);
    let mut channel = dom.get_scheduler_channel();
    assert!(dom.has_work());

    let edits = dom.rebuild();
    assert_eq!(
        edits.edits,
        [
            CreateTextNode { text: "1", root: Some(1) },
            CreateTextNode { text: "2", root: Some(2) },
            CreateTextNode { text: "3", root: Some(3) },
            CreateElement { tag: "div", root: Some(4), children: 0 },
            CreateTextNode { text: "hi", root: Some(5) },
            AppendChildren { root: Some(4), children: vec![5] },
            AppendChildren { root: Some(0), children: vec![1, 2, 3, 4] },
        ]
    )
}
