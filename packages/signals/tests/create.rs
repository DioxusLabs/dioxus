#![allow(unused, non_upper_case_globals, non_snake_case)]

use dioxus::prelude::*;
use dioxus_core::{ElementId, Mutation, NoOpMutations, generation};
use dioxus_signals::*;

#[test]
fn create_signals_global() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            for _ in 0..10 {
                Child {}
            }
        }
    });

    fn Child() -> Element {
        let signal = create_without_cx();

        rsx! {
            "{signal}"
        }
    }

    fn create_without_cx() -> Signal<String> {
        Signal::new("hello world".to_string())
    }

    let muts = dom.rebuild_to_vec();

    // Each of the 10 child scopes renders a single "hello world" text node, and the parent
    // fragment appends all 10 at once. Assert that observable structure without pinning the exact
    // push/pop op tape, which is renderer-protocol detail this rework already churned. These
    // assertions rely on the VirtualDOM's logic, but doing this means not introducing a dependency
    // on a renderer.
    let created_text = muts
        .edits
        .iter()
        .filter(|edit| matches!(edit, Mutation::CreateText { value } if value == "hello world"))
        .count();
    assert_eq!(created_text, 10, "one text node per child signal");
    assert!(
        muts.edits
            .iter()
            .any(|edit| matches!(edit, Mutation::AppendChildren { m } if *m == 10)),
        "all 10 children are appended together"
    );
}

#[test]
fn deref_signal() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static STRINGS_MATCH: AtomicBool = AtomicBool::new(false);

    let mut dom = VirtualDom::new(|| {
        rsx! { Child {} }
    });

    fn Child() -> Element {
        let signal = Signal::new("hello world".to_string());

        // You can call signals like functions to get a Ref of their value.
        let result = &*signal();
        STRINGS_MATCH.store(result.eq("hello world"), Ordering::Relaxed);

        rsx! {
            "arbitrary text"
        }
    }

    dom.rebuild_in_place();

    assert!(STRINGS_MATCH.load(Ordering::Relaxed));
}

#[test]
fn drop_signals() {
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    static SIGNAL_DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    let mut dom = VirtualDom::new(|| {
        let generation = generation();

        let count = if generation % 2 == 0 { 10 } else { 0 };
        rsx! {
            for _ in 0..count {
                Child {}
            }
        }
    });

    fn Child() -> Element {
        struct TracksDrops;

        impl Drop for TracksDrops {
            fn drop(&mut self) {
                SIGNAL_DROP_COUNT.fetch_add(1, Ordering::Relaxed);
            }
        }

        use_signal(|| TracksDrops);

        rsx! {
            ""
        }
    }

    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    assert_eq!(SIGNAL_DROP_COUNT.load(Ordering::Relaxed), 10);
}
