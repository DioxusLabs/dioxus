#![allow(non_snake_case)]

//! Regression tests for <https://github.com/DioxusLabs/dioxus/issues/1929>: `Element`s passed as
//! props are memoized by value, so a component receiving equivalent children from a re-rendered
//! parent does not re-render.

use dioxus::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn static_children_are_memoized() {
    static PARENT_RUNS: AtomicUsize = AtomicUsize::new(0);
    static CHILD_RUNS: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        PARENT_RUNS.fetch_add(1, Ordering::SeqCst);
        rsx! {
            Takes {
                div { "hello" }
            }
        }
    }

    #[component]
    fn Takes(children: Element) -> Element {
        CHILD_RUNS.fetch_add(1, Ordering::SeqCst);
        rsx! {
            div { {children} }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    assert_eq!(PARENT_RUNS.load(Ordering::SeqCst), 1);
    assert_eq!(CHILD_RUNS.load(Ordering::SeqCst), 1);

    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert_eq!(PARENT_RUNS.load(Ordering::SeqCst), 2);
    assert_eq!(CHILD_RUNS.load(Ordering::SeqCst), 1);
}

#[test]
fn unchanged_dynamic_children_are_memoized() {
    static CHILD_RUNS: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        let value = 42;
        rsx! {
            Takes {
                div { "{value}" }
            }
        }
    }

    #[component]
    fn Takes(children: Element) -> Element {
        CHILD_RUNS.fetch_add(1, Ordering::SeqCst);
        rsx! {
            div { {children} }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert_eq!(CHILD_RUNS.load(Ordering::SeqCst), 1);
}

#[test]
fn changed_children_rerun_the_child() {
    static PARENT_RUNS: AtomicUsize = AtomicUsize::new(0);
    static CHILD_RUNS: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        let value = PARENT_RUNS.fetch_add(1, Ordering::SeqCst);
        rsx! {
            Takes {
                div { "{value}" }
            }
        }
    }

    #[component]
    fn Takes(children: Element) -> Element {
        CHILD_RUNS.fetch_add(1, Ordering::SeqCst);
        rsx! {
            div { {children} }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    assert_eq!(CHILD_RUNS.load(Ordering::SeqCst), 1);

    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert_eq!(PARENT_RUNS.load(Ordering::SeqCst), 2);
    assert_eq!(CHILD_RUNS.load(Ordering::SeqCst), 2);
}

#[test]
fn children_with_listeners_are_not_memoized() {
    static CHILD_RUNS: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        rsx! {
            Takes {
                button { onclick: move |_| {}, "click me" }
            }
        }
    }

    #[component]
    fn Takes(children: Element) -> Element {
        CHILD_RUNS.fetch_add(1, Ordering::SeqCst);
        rsx! {
            div { {children} }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert_eq!(CHILD_RUNS.load(Ordering::SeqCst), 2);
}

#[test]
fn nested_components_with_equal_props_are_memoized() {
    static CHILD_RUNS: AtomicUsize = AtomicUsize::new(0);
    static LEAF_RUNS: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        rsx! {
            Takes {
                Leaf { label: "hi" }
            }
        }
    }

    #[component]
    fn Takes(children: Element) -> Element {
        CHILD_RUNS.fetch_add(1, Ordering::SeqCst);
        rsx! {
            div { {children} }
        }
    }

    #[component]
    fn Leaf(label: String) -> Element {
        LEAF_RUNS.fetch_add(1, Ordering::SeqCst);
        rsx! {
            p { "{label}" }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    assert_eq!(CHILD_RUNS.load(Ordering::SeqCst), 1);
    assert_eq!(LEAF_RUNS.load(Ordering::SeqCst), 1);

    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert_eq!(CHILD_RUNS.load(Ordering::SeqCst), 1);
    assert_eq!(LEAF_RUNS.load(Ordering::SeqCst), 1);
}

#[test]
fn nested_components_with_changed_props_rerun() {
    static PARENT_RUNS: AtomicUsize = AtomicUsize::new(0);
    static CHILD_RUNS: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        let value = PARENT_RUNS.fetch_add(1, Ordering::SeqCst);
        rsx! {
            Takes {
                Leaf { label: "{value}" }
            }
        }
    }

    #[component]
    fn Takes(children: Element) -> Element {
        CHILD_RUNS.fetch_add(1, Ordering::SeqCst);
        rsx! {
            div { {children} }
        }
    }

    #[component]
    fn Leaf(label: String) -> Element {
        rsx! {
            p { "{label}" }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert_eq!(CHILD_RUNS.load(Ordering::SeqCst), 2);
}

#[test]
fn vnode_partial_eq_compares_by_value() {
    let value = "x";
    let a = rsx! { div { class: "{value}", "hello" } };
    let b = rsx! { div { class: "{value}", "hello" } };
    let c = rsx! { div { class: "y", "hello" } };
    let d = rsx! { div { class: "{value}", "goodbye" } };

    assert_eq!(a, a);
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(a, d);
}

#[test]
fn vnode_ptr_eq_compares_by_identity() {
    let a = rsx! { div { "hello" } }.unwrap();
    let clone = a.clone();
    let b = rsx! { div { "hello" } }.unwrap();

    assert!(a.ptr_eq(&clone));
    assert!(!a.ptr_eq(&b));
    assert_eq!(a, b);
}

#[test]
fn fragments_and_keyed_lists_compare_by_value() {
    let make = |labels: &[&str]| {
        let labels = labels.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        rsx! {
            ul {
                for label in labels {
                    li { key: "{label}", "{label}" }
                }
            }
        }
    };

    assert_eq!(make(&["a", "b"]), make(&["a", "b"]));
    assert_ne!(make(&["a", "b"]), make(&["a", "c"]));
    assert_ne!(make(&["a", "b"]), make(&["a"]));
}
