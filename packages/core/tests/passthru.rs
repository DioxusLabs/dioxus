#![allow(unused, non_upper_case_globals)]

//! Diffing Tests
//!
//! These tests only verify that the diffing algorithm works properly for single components.
//!
//! It does not validated that component lifecycles work properly. This is done in another test file.

use dioxus::{prelude::*, DomEdit, ScopeId};
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

mod test_logging;

fn new_dom() -> VirtualDom {
    const IS_LOGGING_ENABLED: bool = false;
    test_logging::set_up_logging(IS_LOGGING_ENABLED);
    VirtualDom::new(|cx| rsx!(cx, "hi"))
}

use DomEdit::*;

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
            CreateElement { tag: "div", root: 1 },
            CreateTextNode { text: "hi", root: 2 },
            AppendChildren { many: 1 },
            AppendChildren { many: 1 },
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
            CreateTextNode { text: "1", root: 1 },
            CreateTextNode { text: "2", root: 2 },
            CreateTextNode { text: "3", root: 3 },
            CreateElement { tag: "div", root: 4 },
            CreateTextNode { text: "hi", root: 5 },
            AppendChildren { many: 1 },
            AppendChildren { many: 4 },
        ]
    )
}
