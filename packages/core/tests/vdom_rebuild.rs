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
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

#[test]
fn app_runs() {
    static App: FC<()> = |(cx, props)| {
        //
        rsx!( div{"hello"} )
    };
    let mut vdom = VirtualDom::new(App);
    let edits = vdom.rebuild();
    dbg!(edits);
}

#[test]
fn fragments_work() {
    static App: FC<()> = |(cx, props)| {
        rsx!(
            div{"hello"}
            div{"goodbye"}
        )
    };
    let mut vdom = VirtualDom::new(App);
    let edits = vdom.rebuild();
    // should result in a final "appendchildren n=2"
    dbg!(edits);
}

#[test]
fn lists_work() {
    static App: FC<()> = |(cx, props)| {
        rsx!(
            h1 {"hello"}
            {(0..6).map(|f| rsx!(span{ "{f}" }))}
        )
    };
    let mut vdom = VirtualDom::new(App);
    let edits = vdom.rebuild();
    dbg!(edits);
}

#[test]
fn conditional_rendering() {
    static App: FC<()> = |(cx, props)| {
        rsx!(
            h1 {"hello"}
            {true.then(|| rsx!(span{ "a" }))}
            {false.then(|| rsx!(span{ "b" }))}
        )
    };
    let mut vdom = VirtualDom::new(App);

    let mutations = vdom.rebuild();
    dbg!(&mutations);
    // the "false" fragment should generate an empty placeholder to re-visit
    assert!(mutations.edits[mutations.edits.len() - 2].is("CreatePlaceholder"));
}

#[test]
fn child_components() {
    static App: FC<()> = |(cx, props)| {
        rsx!(
            {true.then(|| rsx!(Child { }))}
            {false.then(|| rsx!(Child { }))}
        )
    };
    static Child: FC<()> = |(cx, props)| {
        rsx!(
            h1 {"hello"}
            h1 {"goodbye"}
        )
    };
    let mut vdom = VirtualDom::new(App);
    let edits = vdom.rebuild();
    dbg!(edits);
}

#[test]
fn suspended_works() {
    static App: FC<()> = |(cx, props)| {
        let title = use_suspense(cx, || async { "bob" }, |cx, f| rsx! { "{f}"});
        rsx!("hello" { title })
    };

    let mut vdom = VirtualDom::new(App);
    let edits = vdom.rebuild();
    dbg!(edits);
}
