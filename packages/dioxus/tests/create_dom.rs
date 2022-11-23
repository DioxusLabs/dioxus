#![allow(unused, non_upper_case_globals, non_snake_case)]

//! Prove that the dom works normally through virtualdom methods.
//!
//! This methods all use "rebuild" which completely bypasses the scheduler.
//! Hard rebuilds don't consume any events from the event queue.

use dioxus::prelude::*;

use dioxus_edit_stream::DomEdit::*;

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
            // create template
            CreateElement { root: Some(1), tag: "template", children: 1 },
            CreateElement { root: None, tag: "div", children: 1 },
            CreateElement { root: None, tag: "div", children: 1 },
            CreateTextNode { root: None, text: "Hello, world!" },
            // clone template
            CloneNodeChildren { id: Some(1), new_ids: vec![2] },
            // add to root
            AppendChildren { root: Some(0), children: vec![2] },
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
            // create template
            CreateElement { root: Some(1), tag: "template", children: 1 },
            CreateElement { root: None, tag: "div", children: 1 },
            CreateElement { root: None, tag: "div", children: 2 },
            CreateTextNode { root: None, text: "Hello, world!" },
            CreateElement { root: None, tag: "div", children: 1 },
            CreateElement { root: None, tag: "div", children: 0 },
            // clone template
            CloneNodeChildren { id: Some(1), new_ids: vec![2] },
            SetLastNode { id: 2 },
            FirstChild {},
            StoreWithId { id: 3 },
            FirstChild {},
            NextSibling {},
            StoreWithId { id: 4 },
            FirstChild {},
            StoreWithId { id: 5 },
            CreateTextNode { root: Some(6), text: "hello" },
            CreateTextNode { root: Some(7), text: "world" },
            SetLastNode { id: 5 },
            AppendChildren { root: None, children: vec![6, 7] },
            AppendChildren { root: Some(0), children: vec![2] }
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

    assert_eq!(
        mutations.edits,
        [
            // create template
            CreateElement { root: Some(1), tag: "template", children: 1 },
            CreateElement { root: None, tag: "div", children: 1 },
            CreateTextNode { root: None, text: "hello" },
            // clone template
            CloneNodeChildren { id: Some(1), new_ids: vec![2] },
            CloneNodeChildren { id: Some(1), new_ids: vec![3] },
            CloneNodeChildren { id: Some(1), new_ids: vec![4] },
            // add to root
            AppendChildren { root: Some(0), children: vec![2, 3, 4] },
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

    assert_eq!(
        mutations.edits,
        [
            // create template
            CreateElement { root: Some(1), tag: "template", children: 4 },
            CreateElement { root: None, tag: "div", children: 0 },
            CreateElement { root: None, tag: "div", children: 0 },
            CreateElement { root: None, tag: "div", children: 0 },
            CreateElement { root: None, tag: "div", children: 0 },
            // clone template
            CloneNodeChildren { id: Some(1), new_ids: vec![2, 3, 4, 5] },
            // add to root
            AppendChildren { root: Some(0), children: vec![2, 3, 4, 5] },
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
            // create template
            CreateElement { root: Some(1), tag: "template", children: 3 },
            CreateElement { root: None, tag: "h1", children: 0 },
            CreateElement { root: None, tag: "div", children: 0 },
            CreateElement { root: None, tag: "p", children: 0 },
            // clone template
            CloneNodeChildren { id: Some(1), new_ids: vec![2, 3, 4] },
            // update template
            SetLastNode { id: 2 },
            NextSibling {},
            CreateTextNode { root: Some(5), text: "abc1" },
            SetLastNode { id: 3 },
            AppendChildren { root: None, children: vec![5] },
            // clone template
            CloneNodeChildren { id: Some(1), new_ids: vec![6, 7, 8] },
            SetLastNode { id: 6 },
            NextSibling {},
            // update template
            CreateTextNode { root: Some(9), text: "abc2" },
            SetLastNode { id: 7 },
            AppendChildren { root: None, children: vec![9] },
            // clone template
            CloneNodeChildren { id: Some(1), new_ids: vec![10, 11, 12] },
            // update template
            SetLastNode { id: 10 },
            NextSibling {},
            CreateTextNode { root: Some(13), text: "abc3" },
            SetLastNode { id: 11 },
            AppendChildren { root: None, children: vec![13] },
            // add to root
            AppendChildren { root: Some(0), children: vec![2, 3, 4, 6, 7, 8, 10, 11, 12] }
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
            // create template
            CreateElement { root: Some(1), tag: "template", children: 1 },
            CreateElement { root: None, tag: "div", children: 1 },
            CreateTextNode { root: None, text: "hello" },
            // clone template
            CloneNodeChildren { id: Some(1), new_ids: vec![2] },
            CreatePlaceholder { root: Some(3) },
            // add to root
            AppendChildren { root: Some(0), children: vec![2, 3] },
        ]
    );
}
