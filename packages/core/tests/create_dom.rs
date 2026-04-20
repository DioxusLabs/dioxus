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

    let edits = dom.rebuild_to_vec();

    assert_eq!(
        edits.edits,
        [
            // add to root
            LoadTemplate { index: 0, id: ElementId(1) },
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

    let _edits = dom.rebuild_to_vec();

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
    //         SaveTemplate {  m: 1 },
    //         // The fragment child template
    //         CreateStaticText { value: "hello" },
    //         CreateStaticText { value: "world" },
    //         SaveTemplate {  m: 2 },
    //     ]
    // );
}

#[test]
fn create_list() {
    let mut dom = VirtualDom::new(|| rsx! {{(0..3).map(|f| rsx!( div { "hello" } ))}});

    let _edits = dom.rebuild_to_vec();

    // note: we dont test template edits anymore
    // assert_eq!(
    //     edits.templates,
    //     [
    //         // create template
    //         CreateElement { name: "div" },
    //         CreateStaticText { value: "hello" },
    //         AppendChildren { m: 1 },
    //         SaveTemplate {  m: 1 }
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

    let edits = dom.rebuild_to_vec();

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
    //         SaveTemplate {  m: 4 }
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

    let _edits = dom.rebuild_to_vec();

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
    let edits = dom.rebuild_to_vec();

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
            LoadTemplate { index: 0, id: ElementId(1) },
            CreatePlaceholder { id: ElementId(2) },
            AppendChildren { m: 2, id: ElementId(0) }
        ]
    )
}

/// Regression test: `create_scope_dom` is used by hydration mismatch recovery to
/// emit creation mutations for a scope whose vdom tree already exists (because
/// `rebuild` was run with skip_mutations to populate state before hydrating).
///
/// The second pass must reuse the element ids that were claimed on the first
/// pass instead of allocating fresh ids — otherwise the arena grows by one slot
/// per element on every recovery and the mutations emitted reference ids that
/// drift from the ones a fresh rebuild would have produced.
#[test]
fn create_scope_dom_reuses_element_ids() {
    fn app() -> Element {
        rsx! {
            div {
                id: "app",
                "hello"
                button { onclick: |_| {}, "click" }
                div { "child" }
            }
        }
    }

    // Baseline: fresh rebuild's mutations.
    let mut baseline = VirtualDom::new(app);
    let baseline_edits = baseline.rebuild_to_vec();

    // Recovery path: rebuild with NoOp (as the web hydration flow does with
    // skip_mutations), then emit creation mutations via `create_scope_dom`.
    let mut recovered = VirtualDom::new(app);
    recovered.rebuild(&mut dioxus_core::NoOpMutations);
    let mut recovered_edits = dioxus_core::Mutations::default();
    let m = recovered
        .create_scope_dom(&mut recovered_edits, ScopeId::ROOT)
        .expect("scope has a rendered tree");
    recovered_edits.edits.push(AppendChildren {
        m,
        id: ElementId(0),
    });

    assert_eq!(
        recovered_edits.edits, baseline_edits.edits,
        "create_scope_dom after a skip_mutations rebuild must emit the same \
         mutations (including element ids) as a fresh rebuild"
    );
}
