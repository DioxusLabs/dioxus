#![allow(unused, non_upper_case_globals, non_snake_case)]

use dioxus::prelude::*;
use dioxus_core::{generation, ElementId, Mutation, NoOpMutations};
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

    // 11 edits: 10x CreateTextNode and 1x AppendChildren. These assertions rely on the VirtualDOM's
    // logic, but doing this means not introducing a dependency on a renderer.
    assert_eq!(11, muts.edits.len());
    for i in 0..10 {
        assert_eq!(
            &muts.edits[i],
            &Mutation::CreateTextNode {
                value: ("hello world".to_string()),
                id: ElementId(i + 1)
            }
        );
    }
    assert_eq!(
        &muts.edits[10],
        &Mutation::AppendChildren {
            id: ElementId(0),
            m: 10
        }
    )
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
