//! Prove that the dom works normally through virtualdom methods.
//! This methods all use "rebuild" which completely bypasses the scheduler.
//! Hard rebuilds don't consume any events from the event queue.

use dioxus::{prelude::*, DomEdit};
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;

mod test_logging;
use DomEdit::*;

fn new_dom<P: Properties + 'static>(app: FC<P>, props: P) -> VirtualDom {
    const IS_LOGGING_ENABLED: bool = false;
    test_logging::set_up_logging(IS_LOGGING_ENABLED);
    VirtualDom::new_with_props(app, props)
}

#[test]
fn test_original_diff() {
    static APP: FC<()> = |cx| {
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
            CreateElement { id: 0, tag: "div" },
            CreateElement { id: 1, tag: "div" },
            CreateTextNode {
                id: 2,
                text: "Hello, world!"
            },
            AppendChildren { many: 1 },
            AppendChildren { many: 1 },
            AppendChildren { many: 1 },
        ]
    );
}

#[async_std::test]
async fn create() {
    static APP: FC<()> = |cx| {
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
            CreateElement { id: 0, tag: "div" },
            CreateElement { id: 1, tag: "div" },
            CreateTextNode {
                id: 2,
                text: "Hello, world!"
            },
            CreateElement { id: 3, tag: "div" },
            CreateElement { id: 4, tag: "div" },
            CreateTextNode {
                id: 5,
                text: "hello"
            },
            CreateTextNode {
                id: 6,
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

#[async_std::test]
async fn create_list() {
    static APP: FC<()> = |cx| {
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
            CreateElement { id: 0, tag: "div" },
            CreateTextNode {
                id: 1,
                text: "hello"
            },
            AppendChildren { many: 1 },
            CreateElement { id: 2, tag: "div" },
            CreateTextNode {
                id: 3,
                text: "hello"
            },
            AppendChildren { many: 1 },
            CreateElement { id: 4, tag: "div" },
            CreateTextNode {
                id: 5,
                text: "hello"
            },
            AppendChildren { many: 1 },
            AppendChildren { many: 3 },
        ]
    );
}

#[async_std::test]
async fn create_simple() {
    static APP: FC<()> = |cx| {
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
            CreateElement { id: 0, tag: "div" },
            CreateElement { id: 1, tag: "div" },
            CreateElement { id: 2, tag: "div" },
            CreateElement { id: 3, tag: "div" },
            AppendChildren { many: 4 },
        ]
    );
}

#[async_std::test]
async fn create_components() {
    static App: FC<()> = |cx| {
        cx.render(rsx! {
            Child { "abc1" }
            Child { "abc2" }
            Child { "abc3" }
        })
    };

    static Child: FC<()> = |cx| {
        cx.render(rsx! {
            h1 {}
            div { {cx.children()} }
            p {}
        })
    };

    let mut dom = new_dom(App, ());
    let mutations = dom.rebuild();

    assert_eq!(
        mutations.edits,
        [
            CreateElement { id: 0, tag: "h1" },
            CreateElement { id: 1, tag: "div" },
            CreateTextNode {
                id: 2,
                text: "abc1"
            },
            AppendChildren { many: 1 },
            CreateElement { id: 3, tag: "p" },
            CreateElement { id: 4, tag: "h1" },
            CreateElement { id: 5, tag: "div" },
            CreateTextNode {
                id: 6,
                text: "abc2"
            },
            AppendChildren { many: 1 },
            CreateElement { id: 7, tag: "p" },
            CreateElement { id: 8, tag: "h1" },
            CreateElement { id: 9, tag: "div" },
            CreateTextNode {
                id: 10,
                text: "abc3"
            },
            AppendChildren { many: 1 },
            CreateElement { id: 11, tag: "p" },
            AppendChildren { many: 9 },
        ]
    );
}

#[async_std::test]
async fn anchors() {
    static App: FC<()> = |cx| {
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
            CreateElement { id: 0, tag: "div" },
            CreateTextNode {
                id: 1,
                text: "hello"
            },
            AppendChildren { many: 1 },
            CreatePlaceholder { id: 2 },
            AppendChildren { many: 2 },
        ]
    );
}

#[async_std::test]
async fn suspended() {
    static App: FC<()> = |cx| {
        let val = use_suspense(
            cx,
            || async {
                //
            },
            |cx, _| cx.render(rsx! { "hi "}),
        );
        cx.render(rsx! { {val} })
    };

    let mut dom = new_dom(App, ());
    let mutations = dom.rebuild();

    assert_eq!(
        mutations.edits,
        [CreatePlaceholder { id: 0 }, AppendChildren { many: 1 },]
    );
}
