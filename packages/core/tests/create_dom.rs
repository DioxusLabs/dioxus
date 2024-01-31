#![allow(unused, non_upper_case_globals, non_snake_case)]

//! Prove that the dom works normally through virtualdom methods.
//!
//! This methods all use "rebuild_to_vec" which completely bypasses the scheduler.
//! Hard rebuild_to_vecs don't consume any events from the event queue.

use dioxus::dioxus_core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::ElementId;

#[test]
fn test_original_diff() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            div { div { "Hello, world!" } }
        }
    });

    let edits = dom.rebuild_to_vec().santize();

    assert_eq!(
        edits.edits,
        [
            // add to root
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            AppendChildren { m: 1, id: ElementId(0) }
        ]
    )
}

#[test]
fn create() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            div {
                div {
                    "Hello, world!"
                    div {
                        div {
                            Fragment { "hello""world" }
                        }
                    }
                }
            }
        }
    });

    let _edits = dom.rebuild_to_vec().santize();

    // todo: we don't test template mutations anymore since the templates are passed along

    // assert_eq!(
    //     edits.templates,
    //     [
    //         // create template
    //         CreateElement { name: "div" },
    //         CreateElement { name: "div" },
    //         CreateStaticText { value: "Hello, world!" },
    //         CreateElement { name: "div" },
    //         CreateElement { name: "div" },
    //         CreateStaticPlaceholder {},
    //         AppendChildren { m: 1 },
    //         AppendChildren { m: 1 },
    //         AppendChildren { m: 2 },
    //         AppendChildren { m: 1 },
    //         SaveTemplate { name: "template", m: 1 },
    //         // The fragment child template
    //         CreateStaticText { value: "hello" },
    //         CreateStaticText { value: "world" },
    //         SaveTemplate { name: "template", m: 2 },
    //     ]
    // );
}

#[test]
fn create_list() {
    let mut dom = VirtualDom::new(|| rsx! {{(0..3).map(|f| rsx!( div { "hello" } ))}});

    let _edits = dom.rebuild_to_vec().santize();

    // note: we dont test template edits anymore
    // assert_eq!(
    //     edits.templates,
    //     [
    //         // create template
    //         CreateElement { name: "div" },
    //         CreateStaticText { value: "hello" },
    //         AppendChildren { m: 1 },
    //         SaveTemplate { name: "template", m: 1 }
    //     ]
    // );
}

#[test]
fn create_simple() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            div {}
            div {}
            div {}
            div {}
        }
    });

    let edits = dom.rebuild_to_vec().santize();

    // note: we dont test template edits anymore
    // assert_eq!(
    //     edits.templates,
    //     [
    //         // create template
    //         CreateElement { name: "div" },
    //         CreateElement { name: "div" },
    //         CreateElement { name: "div" },
    //         CreateElement { name: "div" },
    //         // add to root
    //         SaveTemplate { name: "template", m: 4 }
    //     ]
    // );
}
#[test]
fn create_components() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            Child { "abc1" }
            Child { "abc2" }
            Child { "abc3" }
        }
    });

    #[derive(Props, Clone, PartialEq)]
    struct ChildProps {
        children: Element,
    }

    fn Child(cx: ChildProps) -> Element {
        rsx! {
            h1 {}
            div { {cx.children} }
            p {}
        }
    }

    let _edits = dom.rebuild_to_vec().santize();

    // todo: test this
}

#[test]
fn anchors() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            if true {
                 div { "hello" }
            }
            if false {
                div { "goodbye" }
            }
        }
    });

    // note that the template under "false" doesn't show up since it's not loaded
    let edits = dom.rebuild_to_vec().santize();

    // note: we dont test template edits anymore
    // assert_eq!(
    //     edits.templates,
    //     [
    //         // create each template
    //         CreateElement { name: "div" },
    //         CreateStaticText { value: "hello" },
    //         AppendChildren { m: 1 },
    //         SaveTemplate { m: 1, name: "template" },
    //     ]
    // );

    assert_eq!(
        edits.edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            CreatePlaceholder { id: ElementId(2) },
            AppendChildren { m: 2, id: ElementId(0) }
        ]
    )
}
