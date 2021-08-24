//! tests to prove that the iterative implementation works

use anyhow::{Context, Result};
use dioxus::{arena::SharedResources, diff::DiffMachine, prelude::*, DomEdit, Mutations};
mod test_logging;
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;
use DomEdit::*;

const LOGGING_ENABLED: bool = false;

#[test]
fn test_original_diff() {
    static App: FC<()> = |cx| {
        cx.render(rsx! {
            div {
                div {
                    "Hello, world!"
                }
            }
        })
    };

    let mut dom = VirtualDom::new(App);
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
    static App: FC<()> = |cx| {
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

    test_logging::set_up_logging(LOGGING_ENABLED);
    let mut dom = VirtualDom::new(App);
    let mutations = dom.rebuild_async().await;
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
    static App: FC<()> = |cx| {
        cx.render(rsx! {
            {(0..3).map(|f| rsx!{ div {
                "hello"
            }})}
        })
    };

    test_logging::set_up_logging(LOGGING_ENABLED);

    let mut dom = VirtualDom::new(App);
    let mutations = dom.rebuild_async().await;

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
    static App: FC<()> = |cx| {
        cx.render(rsx! {
            div {}
            div {}
            div {}
            div {}
        })
    };

    test_logging::set_up_logging(LOGGING_ENABLED);

    let mut dom = VirtualDom::new(App);
    let mutations = dom.rebuild_async().await;

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

    test_logging::set_up_logging(LOGGING_ENABLED);

    let mut dom = VirtualDom::new(App);
    let mutations = dom.rebuild_async().await;

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

    test_logging::set_up_logging(LOGGING_ENABLED);

    let mut dom = VirtualDom::new(App);
    let mutations = dom.rebuild_async().await;
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
        let val = use_suspense(cx, || async {}, |cx, _| cx.render(rsx! { "hi "}));
        cx.render(rsx! { {val} })
    };

    test_logging::set_up_logging(LOGGING_ENABLED);

    let mut dom = VirtualDom::new(App);
    let mutations = dom.rebuild_async().await;

    assert_eq!(
        mutations.edits,
        [CreatePlaceholder { id: 0 }, AppendChildren { many: 1 },]
    );
}
