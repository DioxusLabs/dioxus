#![allow(unused, non_upper_case_globals)]

//! Rebuilding tests
//! ----------------
//!
//! This tests module ensures that the initial build of the virtualdom is correct.
//! This does not include dynamic tests or the diffing algorithm itself.
//!
//! It does prove that mounting works properly and the correct edit streams are generated.
//!
//! Don't have a good way to validate, everything is done manually ATM

use dioxus::prelude::*;
use dioxus_core::DomEdit::*;

#[test]
fn app_runs() {
    static App: Component = |cx| render!(div{"hello"} );

    let mut vdom = VirtualDom::new(App);
    let edits = vdom.rebuild();
}

#[test]
fn fragments_work() {
    static App: Component = |cx| {
        cx.render(rsx!(
            div{"hello"}
            div{"goodbye"}
        ))
    };
    let mut vdom = VirtualDom::new(App);
    let edits = vdom.rebuild();
    // should result in a final "appendchildren n=2"
    dbg!(edits);
}

#[test]
fn lists_work() {
    static App: Component = |cx| {
        cx.render(rsx!(
            h1 {"hello"}
            (0..6).map(|f| rsx!(span{ "{f}" }))
        ))
    };
    let mut vdom = VirtualDom::new(App);
    let edits = vdom.rebuild();
    dbg!(edits);
}

#[test]
fn conditional_rendering() {
    static App: Component = |cx| {
        cx.render(rsx!(
            h1 {"hello"}
            {true.then(|| rsx!(span{ "a" }))}
            {false.then(|| rsx!(span{ "b" }))}
        ))
    };
    let mut vdom = VirtualDom::new(App);

    let mutations = vdom.rebuild();
    assert_eq!(
        mutations.edits,
        [
            CreateElement { root: Some(1), tag: "template", children: 3 },
            CreateElement { root: None, tag: "h1", children: 1 },
            CreateTextNode { root: None, text: "hello" },
            CreatePlaceholder { root: None },
            CreatePlaceholder { root: None },
            CloneNodeChildren { id: Some(1), new_ids: vec![2, 3, 4] },
            CreateElement { root: Some(5), tag: "template", children: 1 },
            CreateElement { root: None, tag: "span", children: 1 },
            CreateTextNode { root: None, text: "a" },
            CloneNodeChildren { id: Some(5), new_ids: vec![6] },
            SetLastNode { id: 3 },
            ReplaceWith { root: None, nodes: vec![6] },
            CreatePlaceholder { root: Some(7) },
            SetLastNode { id: 4 },
            ReplaceWith { root: None, nodes: vec![7] },
            AppendChildren { root: Some(0), children: vec![2, 3, 4] }
        ]
    )
}

#[test]
fn child_components() {
    static App: Component = |cx| {
        cx.render(rsx!(
            {true.then(|| rsx!(Child { }))}
            {false.then(|| rsx!(Child { }))}
        ))
    };
    static Child: Component = |cx| {
        cx.render(rsx!(
            h1 {"hello"}
            h1 {"goodbye"}
        ))
    };
    let mut vdom = VirtualDom::new(App);
    let edits = vdom.rebuild();
    dbg!(edits);
}
