#![allow(non_snake_case)]

//! Tests for memoizing components by the *value* of their `Element` props.
//!
//! When a component re-renders and passes an `Element` to a child component,
//! the `Element` is rebuilt from scratch: it is a new allocation, so pointer
//! equality fails. The child should still be able to skip re-rendering when the
//! new `Element` has the same content as the previous one.
//!
//! See https://github.com/DioxusLabs/dioxus/issues/1929.

use dioxus::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn element_props_memoize_by_value() {
    static WHATEVER_RENDERS: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        rsx! {
            div {
                Whatever {
                    div { "static children" }
                }
            }
        }
    }

    #[component]
    fn Whatever(children: Element) -> Element {
        WHATEVER_RENDERS.fetch_add(1, Ordering::Relaxed);
        rsx! { div { {children} } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    assert_eq!(WHATEVER_RENDERS.load(Ordering::Relaxed), 1);

    // Re-render the parent a few times. The `children` element is rebuilt with
    // the same content, so `Whatever` should not re-run even though the
    // `Element` is a new allocation.
    for _ in 0..3 {
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut dioxus_core::NoOpMutations);
    }

    assert_eq!(
        WHATEVER_RENDERS.load(Ordering::Relaxed),
        1,
        "component with unchanged Element props should not re-render"
    );
}

#[test]
fn element_props_rerender_when_value_changes() {
    static WHATEVER_RENDERS: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        let mut render_count = use_signal(|| 0);
        // Advance the counter so the children element's content changes
        // between renders.
        render_count += 1;
        rsx! {
            div {
                Whatever {
                    div { "children {render_count}" }
                }
            }
        }
    }

    #[component]
    fn Whatever(children: Element) -> Element {
        WHATEVER_RENDERS.fetch_add(1, Ordering::Relaxed);
        rsx! { div { {children} } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    assert_eq!(WHATEVER_RENDERS.load(Ordering::Relaxed), 1);

    // Re-render the parent. The children element now has different content, so
    // `Whatever` must re-render.
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert_eq!(
        WHATEVER_RENDERS.load(Ordering::Relaxed),
        2,
        "component with changed Element props should re-render"
    );
}
