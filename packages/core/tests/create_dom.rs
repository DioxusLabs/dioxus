#![allow(unused, non_upper_case_globals, non_snake_case)]

//! Prove that the dom works normally through virtualdom methods.

use dioxus::dioxus_core::Mutation::*;
use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;

#[test]
fn test_original_diff() {
    Sequence::new()
        .render(rsx! { div { div { "Hello, world!" } } })
        .run();
}

#[test]
fn create() {
    Sequence::new()
        .render({
            rsx! {
                div {
                    div {
                        "Hello, world!"
                        div {
                            div {
                                Fragment { "hello" "world" }
                            }
                        }
                    }
                }
            }
        })
        .run();
}

#[test]
fn create_list() {
    fn app() -> Element {
        rsx! {{(0..3).map(|_| rsx!( div { "hello" } ))}}
    }

    Sequence::new().render_with(app).run();
}

#[test]
fn create_simple() {
    Sequence::new()
        .render(rsx! { div {} div {} div {} div {} })
        .run();
}

#[test]
fn create_components() {
    fn app() -> Element {
        rsx! {
            Child { "abc1" }
            Child { "abc2" }
            Child { "abc3" }
        }
    }

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

    Sequence::new().render_with(app).run();
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

    let edits = dom.rebuild_to_vec();

    assert_eq!(edits.edits.len(), 3);
    assert!(matches!(edits.edits[0], LoadTemplate { index: 0, .. }));
    assert!(matches!(edits.edits[1], CreatePlaceholder { .. }));
    assert!(matches!(edits.edits[2], AppendChildren { m: 2, .. }));
}
