#![allow(unused, non_upper_case_globals, non_snake_case)]

//! Prove that the dom works normally through virtualdom methods.
//!
//! This methods all use "rebuild" which completely bypasses the scheduler.
//! Hard rebuilds don't consume any events from the event queue.

use dioxus::prelude::*;

use dioxus_core::DomEdit::*;

fn new_dom<P: 'static + Send>(app: Component<P>, props: P) -> VirtualDom {
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
            CreateTemplate { id: 0 },
            CreateElementTemplate {
                root: 4503599627370495,
                tag: "div",
                locally_static: true,
                fully_static: true
            },
            CreateElementTemplate {
                root: 4503599627370496,
                tag: "div",
                locally_static: true,
                fully_static: true
            },
            CreateTextNodeTemplate {
                root: 4503599627370497,
                text: "Hello, world!",
                locally_static: true
            },
            AppendChildren { many: 1 },
            AppendChildren { many: 1 },
            FinishTemplate { len: 1 },
            CreateTemplateRef { id: 1, template_id: 0 },
            AppendChildren { many: 1 }
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
            CreateTemplate { id: 0 },
            CreateElementTemplate {
                root: 4503599627370495,
                tag: "div",
                locally_static: true,
                fully_static: false
            },
            CreateElementTemplate {
                root: 4503599627370496,
                tag: "div",
                locally_static: true,
                fully_static: false
            },
            CreateTextNodeTemplate {
                root: 4503599627370497,
                text: "Hello, world!",
                locally_static: true
            },
            CreateElementTemplate {
                root: 4503599627370498,
                tag: "div",
                locally_static: true,
                fully_static: false
            },
            CreateElementTemplate {
                root: 4503599627370499,
                tag: "div",
                locally_static: true,
                fully_static: false
            },
            CreatePlaceholderTemplate { root: 4503599627370500 },
            AppendChildren { many: 1 },
            AppendChildren { many: 1 },
            AppendChildren { many: 2 },
            AppendChildren { many: 1 },
            FinishTemplate { len: 1 },
            CreateTemplateRef { id: 1, template_id: 0 },
            EnterTemplateRef { root: 1 },
            CreateTextNode { root: 2, text: "hello" },
            CreateTextNode { root: 3, text: "world" },
            ReplaceWith { root: 4503599627370500, m: 2 },
            ExitTemplateRef {},
            AppendChildren { many: 1 }
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
            CreateTemplate { id: 0 },
            CreateElementTemplate {
                root: 4503599627370495,
                tag: "div",
                locally_static: true,
                fully_static: true
            },
            CreateTextNodeTemplate { root: 4503599627370496, text: "hello", locally_static: true },
            AppendChildren { many: 1 },
            FinishTemplate { len: 1 },
            CreateTemplateRef { id: 1, template_id: 0 },
            CreateTemplateRef { id: 2, template_id: 0 },
            CreateTemplateRef { id: 3, template_id: 0 },
            AppendChildren { many: 3 }
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
            CreateTemplate { id: 0 },
            CreateElementTemplate {
                root: 4503599627370495,
                tag: "div",
                locally_static: true,
                fully_static: true
            },
            AppendChildren { many: 0 },
            CreateElementTemplate {
                root: 4503599627370496,
                tag: "div",
                locally_static: true,
                fully_static: true
            },
            AppendChildren { many: 0 },
            CreateElementTemplate {
                root: 4503599627370497,
                tag: "div",
                locally_static: true,
                fully_static: true
            },
            AppendChildren { many: 0 },
            CreateElementTemplate {
                root: 4503599627370498,
                tag: "div",
                locally_static: true,
                fully_static: true
            },
            AppendChildren { many: 0 },
            FinishTemplate { len: 4 },
            CreateTemplateRef { id: 1, template_id: 0 },
            AppendChildren { many: 1 }
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
            div { &cx.props.children }
            p {}
        })
    }

    let mut dom = new_dom(App, ());
    let mutations = dom.rebuild();

    assert_eq!(
        mutations.edits,
        [
            CreateTemplate { id: 0 },
            CreateElementTemplate {
                root: 4503599627370495,
                tag: "h1",
                locally_static: true,
                fully_static: true
            },
            AppendChildren { many: 0 },
            CreateElementTemplate {
                root: 4503599627370496,
                tag: "div",
                locally_static: true,
                fully_static: false
            },
            CreatePlaceholderTemplate { root: 4503599627370497 },
            AppendChildren { many: 1 },
            CreateElementTemplate {
                root: 4503599627370498,
                tag: "p",
                locally_static: true,
                fully_static: true
            },
            AppendChildren { many: 0 },
            FinishTemplate { len: 3 },
            CreateTemplateRef { id: 1, template_id: 0 },
            EnterTemplateRef { root: 1 },
            CreateTextNode { root: 2, text: "abc1" },
            ReplaceWith { root: 4503599627370497, m: 1 },
            ExitTemplateRef {},
            CreateTemplateRef { id: 3, template_id: 0 },
            EnterTemplateRef { root: 3 },
            CreateTextNode { root: 4, text: "abc2" },
            ReplaceWith { root: 4503599627370497, m: 1 },
            ExitTemplateRef {},
            CreateTemplateRef { id: 5, template_id: 0 },
            EnterTemplateRef { root: 5 },
            CreateTextNode { root: 6, text: "abc3" },
            ReplaceWith { root: 4503599627370497, m: 1 },
            ExitTemplateRef {},
            AppendChildren { many: 3 }
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
            CreateTemplate { id: 0 },
            CreateElementTemplate {
                root: 4503599627370495,
                tag: "div",
                locally_static: true,
                fully_static: true
            },
            CreateTextNodeTemplate { root: 4503599627370496, text: "hello", locally_static: true },
            AppendChildren { many: 1 },
            FinishTemplate { len: 1 },
            CreateTemplateRef { id: 1, template_id: 0 },
            CreatePlaceholder { root: 2 },
            AppendChildren { many: 2 }
        ]
    );
}
