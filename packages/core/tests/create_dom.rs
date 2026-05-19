#![allow(unused, non_upper_case_globals, non_snake_case)]

//! Prove that the dom works normally through virtualdom methods.

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

    Sequence::new()
        .render_with_expected(
            app,
            rsx! {
                div { "hello" }
                div { "hello" }
                div { "hello" }
            },
        )
        .run();
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

    Sequence::new()
        .render_with_expected(
            app,
            rsx! {
                h1 {}
                div { "abc1" }
                p {}
                h1 {}
                div { "abc2" }
                p {}
                h1 {}
                div { "abc3" }
                p {}
            },
        )
        .run();
}

#[test]
fn anchors() {
    fn app() -> Element {
        rsx! {
            if true {
                 div { "hello" }
            }
            if false {
                div { "goodbye" }
            }
        }
    }

    Sequence::new()
        .render_with_expected(
            app,
            rsx! {
                if true {
                     div { "hello" }
                }
                if false {
                    div { "goodbye" }
                }
            },
        )
        .run();
}
