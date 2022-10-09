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
            CreateTemplate { id: 0 },
            CreateElementTemplate {
                root: 4503599627370495,
                tag: "h1",
                locally_static: true,
                fully_static: true
            },
            CreateTextNodeTemplate { root: 4503599627370496, text: "hello", locally_static: true },
            AppendChildren { many: 1 },
            CreatePlaceholderTemplate { root: 4503599627370497 },
            CreatePlaceholderTemplate { root: 4503599627370498 },
            FinishTemplate { len: 3 },
            CreateTemplateRef { id: 1, template_id: 0 },
            EnterTemplateRef { root: 1 },
            CreateTemplate { id: 1 },
            CreateElementTemplate {
                root: 4503599627370495,
                tag: "span",
                locally_static: true,
                fully_static: true
            },
            CreateTextNodeTemplate { root: 4503599627370496, text: "a", locally_static: true },
            AppendChildren { many: 1 },
            FinishTemplate { len: 1 },
            CreateTemplateRef { id: 2, template_id: 1 },
            ReplaceWith { root: 4503599627370497, m: 1 },
            ExitTemplateRef {},
            EnterTemplateRef { root: 1 },
            CreatePlaceholder { root: 3 },
            ReplaceWith { root: 4503599627370498, m: 1 },
            ExitTemplateRef {},
            AppendChildren { many: 1 }
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
