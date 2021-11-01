#![allow(unused, non_upper_case_globals)]

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

fn new_dom<P: 'static + Send>(app: FC<P>, props: P) -> VirtualDom {
    const IS_LOGGING_ENABLED: bool = false;
    test_logging::set_up_logging(IS_LOGGING_ENABLED);
    VirtualDom::new_with_props(app, props)
}

#[test]
fn test_original_diff() {
    static APP: FC<()> = |(cx, props)| {
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
                root: 0,
                tag: "div"
            },
            CreateElement {
                root: 1,
                tag: "div"
            },
            CreateTextNode {
                root: 2,
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
    static APP: FC<()> = |(cx, props)| {
        cx.render(rsx! {
            div {
                div {
                    "Hello, world!"
                    div {
                        div {
                            // Fragment {
                            //     "hello"
                            //     "world"
                            // }
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
                root: 0,
                tag: "div"
            },
            CreateElement {
                root: 1,
                tag: "div"
            },
            CreateTextNode {
                root: 2,
                text: "Hello, world!"
            },
            CreateElement {
                root: 3,
                tag: "div"
            },
            CreateElement {
                root: 4,
                tag: "div"
            },
            CreateTextNode {
                root: 5,
                text: "hello"
            },
            CreateTextNode {
                root: 6,
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
    static APP: FC<()> = |(cx, props)| {
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
                root: 0,
                tag: "div"
            },
            CreateTextNode {
                root: 1,
                text: "hello"
            },
            AppendChildren { many: 1 },
            CreateElement {
                root: 2,
                tag: "div"
            },
            CreateTextNode {
                root: 3,
                text: "hello"
            },
            AppendChildren { many: 1 },
            CreateElement {
                root: 4,
                tag: "div"
            },
            CreateTextNode {
                root: 5,
                text: "hello"
            },
            AppendChildren { many: 1 },
            AppendChildren { many: 3 },
        ]
    );
}

#[test]
fn create_simple() {
    static APP: FC<()> = |(cx, props)| {
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
                root: 0,
                tag: "div"
            },
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
            AppendChildren { many: 4 },
        ]
    );
}
#[test]
fn create_components() {
    static App: FC<()> = |(cx, props)| {
        cx.render(rsx! {
            Child { "abc1" }
            Child { "abc2" }
            Child { "abc3" }
        })
    };

    #[derive(Props)]
    struct ChildProps<'a> {
        children: ScopeChildren<'a>,
    }

    fn Child<'a>((cx, props): Scope<'a, ChildProps<'a>>) -> Element {
        cx.render(rsx! {
            h1 {}
            div { {&props.children} }
            p {}
        })
    }

    let mut dom = new_dom(App, ());
    let mutations = dom.rebuild();

    assert_eq!(
        mutations.edits,
        [
            CreateElement { root: 0, tag: "h1" },
            CreateElement {
                root: 1,
                tag: "div"
            },
            CreateTextNode {
                root: 2,
                text: "abc1"
            },
            AppendChildren { many: 1 },
            CreateElement { root: 3, tag: "p" },
            CreateElement { root: 4, tag: "h1" },
            CreateElement {
                root: 5,
                tag: "div"
            },
            CreateTextNode {
                root: 6,
                text: "abc2"
            },
            AppendChildren { many: 1 },
            CreateElement { root: 7, tag: "p" },
            CreateElement { root: 8, tag: "h1" },
            CreateElement {
                root: 9,
                tag: "div"
            },
            CreateTextNode {
                root: 10,
                text: "abc3"
            },
            AppendChildren { many: 1 },
            CreateElement { root: 11, tag: "p" },
            AppendChildren { many: 9 },
        ]
    );
}
#[test]
fn anchors() {
    static App: FC<()> = |(cx, props)| {
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
                root: 0,
                tag: "div"
            },
            CreateTextNode {
                root: 1,
                text: "hello"
            },
            AppendChildren { many: 1 },
            CreatePlaceholder { root: 2 },
            AppendChildren { many: 2 },
        ]
    );
}

#[test]
fn suspended() {
    static App: FC<()> = |(cx, props)| {
        let val = use_suspense(
            cx,
            || async {
                //
            },
            |cx, p| todo!(),
            // |cx, p| rsx! { "hi "},
        );

        // let prom = use_task(fetch());

        // Value {
        //     a: value.await,
        //     b: value.await,
        //     f: a.await
        //     div {
        //         div {
        //             hidden: should_hide.await,
        //         }
        //     }
        // }
        cx.render(rsx! { {val} })
    };

    let mut dom = new_dom(App, ());
    let mutations = dom.rebuild();

    assert_eq!(
        mutations.edits,
        [CreatePlaceholder { root: 0 }, AppendChildren { many: 1 },]
    );
}
