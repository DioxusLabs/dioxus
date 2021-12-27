#![allow(unused, non_upper_case_globals, non_snake_case)]

//! Prove that the dom works normally through virtualdom methods.
//!
//! This methods all use "rebuild" which completely bypasses the scheduler.
//! Hard rebuilds don't consume any events from the event queue.

use dioxus::{prelude::*, DomEdit};
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

mod test_logging;
use DomEdit::*;

fn new_dom<P: 'static + Send>(app: Component<P>, props: P) -> VirtualDom {
    const IS_LOGGING_ENABLED: bool = false;
    test_logging::set_up_logging(IS_LOGGING_ENABLED);
    VirtualDom::new_with_props(app, props)
}

#[test]
fn test_original_diff() {
    static APP: Component = |cx| {
        cx.render(rsx! {
            div {
                div {
                    "Hello, world!"
                }
            }
        })
    };

    let mut dom = new_dom(APP, ());
    let mutations = dom.rebuild();
    assert_eq!(
        mutations.edits,
        [
            CreateElement {
                root: 1,
                tag: "div"
            },
            CreateElement {
                root: 2,
                tag: "div"
            },
            CreateTextNode {
                root: 3,
                text: "Hello, world!"
            },
            AppendChildren { many: 1 },
            AppendChildren { many: 1 },
            AppendChildren { many: 1 },
        ]
    );
}

#[test]
fn create() {
    static APP: Component = |cx| {
        cx.render(rsx! {
            div {
                div {
                    "Hello, world!"
                    div {
                        div {
                            Fragment {
                                "hello"
                                "world"
                            }
                        }
                    }
                }
            }
        })
    };

    let mut dom = new_dom(APP, ());
    let mutations = dom.rebuild();

    assert_eq!(
        mutations.edits,
        [
            CreateElement {
                root: 1,
                tag: "div"
            },
            CreateElement {
                root: 2,
                tag: "div"
            },
            CreateTextNode {
                root: 3,
                text: "Hello, world!"
            },
            CreateElement {
                root: 4,
                tag: "div"
            },
            CreateElement {
                root: 5,
                tag: "div"
            },
            CreateTextNode {
                root: 6,
                text: "hello"
            },
            CreateTextNode {
                root: 7,
                text: "world"
            },
            AppendChildren { many: 2 },
            AppendChildren { many: 1 },
            AppendChildren { many: 2 },
            AppendChildren { many: 1 },
            AppendChildren { many: 1 },
        ]
    );
}

#[test]
fn create_list() {
    static APP: Component = |cx| {
        cx.render(rsx! {
            {(0..3).map(|f| rsx!{ div {
                "hello"
            }})}
        })
    };

    let mut dom = new_dom(APP, ());
    let mutations = dom.rebuild();

    // copilot wrote this test :P
    assert_eq!(
        mutations.edits,
        [
            CreateElement {
                root: 1,
                tag: "div"
            },
            CreateTextNode {
                root: 2,
                text: "hello"
            },
            AppendChildren { many: 1 },
            CreateElement {
                root: 3,
                tag: "div"
            },
            CreateTextNode {
                root: 4,
                text: "hello"
            },
            AppendChildren { many: 1 },
            CreateElement {
                root: 5,
                tag: "div"
            },
            CreateTextNode {
                root: 6,
                text: "hello"
            },
            AppendChildren { many: 1 },
            AppendChildren { many: 3 },
        ]
    );
}

#[test]
fn create_simple() {
    static APP: Component = |cx| {
        cx.render(rsx! {
            div {}
            div {}
            div {}
            div {}
        })
    };

    let mut dom = new_dom(APP, ());
    let mutations = dom.rebuild();

    // copilot wrote this test :P
    assert_eq!(
        mutations.edits,
        [
            CreateElement {
                root: 1,
                tag: "div"
            },
            CreateElement {
                root: 2,
                tag: "div"
            },
            CreateElement {
                root: 3,
                tag: "div"
            },
            CreateElement {
                root: 4,
                tag: "div"
            },
            AppendChildren { many: 4 },
        ]
    );
}
#[test]
fn create_components() {
    static App: Component = |cx| {
        cx.render(rsx! {
            Child { "abc1" }
            Child { "abc2" }
            Child { "abc3" }
        })
    };

    #[derive(Props)]
    struct ChildProps<'a> {
        children: Element<'a>,
    }

    fn Child<'a>(cx: Scope<'a, ChildProps<'a>>) -> Element {
        cx.render(rsx! {
            h1 {}
            div { {&cx.props.children} }
            p {}
        })
    }

    let mut dom = new_dom(App, ());
    let mutations = dom.rebuild();

    assert_eq!(
        mutations.edits,
        [
            CreateElement { root: 1, tag: "h1" },
            CreateElement {
                root: 2,
                tag: "div"
            },
            CreateTextNode {
                root: 3,
                text: "abc1"
            },
            AppendChildren { many: 1 },
            CreateElement { root: 4, tag: "p" },
            CreateElement { root: 5, tag: "h1" },
            CreateElement {
                root: 6,
                tag: "div"
            },
            CreateTextNode {
                root: 7,
                text: "abc2"
            },
            AppendChildren { many: 1 },
            CreateElement { root: 8, tag: "p" },
            CreateElement { root: 9, tag: "h1" },
            CreateElement {
                root: 10,
                tag: "div"
            },
            CreateTextNode {
                root: 11,
                text: "abc3"
            },
            AppendChildren { many: 1 },
            CreateElement { root: 12, tag: "p" },
            AppendChildren { many: 9 },
        ]
    );
}
#[test]
fn anchors() {
    static App: Component = |cx| {
        cx.render(rsx! {
            {true.then(|| rsx!{ div { "hello" } })}
            {false.then(|| rsx!{ div { "goodbye" } })}
        })
    };

    let mut dom = new_dom(App, ());
    let mutations = dom.rebuild();
    assert_eq!(
        mutations.edits,
        [
            CreateElement {
                root: 1,
                tag: "div"
            },
            CreateTextNode {
                root: 2,
                text: "hello"
            },
            AppendChildren { many: 1 },
            CreatePlaceholder { root: 3 },
            AppendChildren { many: 2 },
        ]
    );
}
