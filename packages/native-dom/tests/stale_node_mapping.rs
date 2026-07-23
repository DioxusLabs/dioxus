//! Regression tests for stale `node_id_mapping` entries after node removal.
//! See https://github.com/DioxusLabs/dioxus/pull/5182
//!
//! When `remove_node` (or `replace_node_with`) detaches a node, the
//! `node_id_mapping` entries for the node's subtree are (intentionally) not
//! cleared, so that dioxus-core can reuse the detached DOM nodes. However,
//! when the detached subtree is later dropped via `remove_node_if_unparented`
//! (called from `assign_node_id`), only the mapping for the ElementId being
//! reassigned is updated. Any *descendant* of the dropped subtree that has its
//! own ElementId mapping is left pointing at a freed slab slot.
//!
//! When that slab slot is reused for a brand-new (not yet parented) node and
//! the stale ElementId is then reused by dioxus-core via `AssignId`,
//! `remove_node_if_unparented` incorrectly drops the new node, causing an
//! "invalid key" slab panic in blitz-dom's `DocumentMutator` later in the
//! same render (matching the stack trace reported in the PR).

use blitz_dom::DocumentConfig;
use blitz_traits::shell::{ColorScheme, Viewport};
use dioxus::prelude::*;
use dioxus_core::ScopeId;
use dioxus_native_dom::DioxusDocument;
use std::cell::Cell;

thread_local! {
    static GENERATION: Cell<usize> = const { Cell::new(0) };
}

fn generation() -> usize {
    GENERATION.with(|g| g.get())
}

fn make_doc(app: fn() -> Element) -> DioxusDocument {
    GENERATION.with(|g| g.set(0));
    let vdom = VirtualDom::new(app);
    let mut doc = DioxusDocument::new(
        vdom,
        DocumentConfig {
            viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
            ..Default::default()
        },
    );
    doc.initial_build();
    doc
}

fn step(doc: &mut DioxusDocument) {
    use blitz_dom::Document as _;
    GENERATION.with(|g| g.set(g.get() + 1));
    doc.vdom.mark_dirty(ScopeId::APP);
    doc.poll(None);
}

/// Check that every ElementId -> NodeId mapping points at a node that still
/// exists in the document. A mapping pointing at a freed slab slot will cause
/// an "invalid key" panic (or worse: silent deletion of an unrelated node
/// that happens to reuse the slot) on a later render.
fn assert_no_stale_mappings(doc: &DioxusDocument, context: &str) {
    let inner = doc.inner.borrow();
    let mut stale = Vec::new();
    for raw_id in 0..4096 {
        let element_id = dioxus_core::ElementId(raw_id);
        if let Some(node_id) = doc.vdom_state.try_element_to_node_id(element_id)
            && inner.get_node(node_id).is_none()
        {
            stale.push((raw_id, node_id));
        }
    }
    assert!(
        stale.is_empty(),
        "stale node_id_mapping entries (ElementId, NodeId) after {context}: {stale:?}"
    );
}

fn bare_text() -> Element {
    let generation = generation();
    rsx! { "bare text {generation}" }
}

fn section_with_list(n: usize) -> Element {
    let generation = generation();
    rsx! {
        section {
            header { id: "gen-{generation}", "header" }
            for i in 0..n {
                div { key: "{i}", "item {i}" }
            }
        }
    }
}

fn nested_divs() -> Element {
    let generation = generation();
    rsx! {
        article {
            div { class: "gen-{generation}",
                div { id: "gen-{generation}", "deep {generation}" }
            }
        }
        aside { "aside" }
    }
}

/// Minimal deterministic sequence of template swaps that panics with
/// "invalid key" inside blitz-dom's `DocumentMutator` (in `node_at_path`
/// via `assign_node_id`) on the 3rd re-render.
fn minimal_panic_app() -> Element {
    match generation() {
        0 => nested_divs(),
        1 => bare_text(),
        2 => section_with_list(0),
        _ => nested_divs(),
    }
}

#[test]
fn template_swaps_do_not_panic() {
    let mut doc = make_doc(minimal_panic_app);
    for _ in 0..3 {
        step(&mut doc);
    }
}

/// Toggles between templates where one variant has a nested element with a
/// dynamic attribute (which is assigned an ElementId via `AssignId`).
/// Detects the stale mapping (the precursor to the panic) directly.
fn toggle_app() -> Element {
    let generation = generation();
    let class = format!("gen-{generation}");
    match generation % 3 {
        0 => rsx! {
            div {
                div { class: "{class}",
                    span { "nested {generation}" }
                }
                "text {generation}"
            }
            div { "sibling" }
        },
        1 => rsx! {
            p { "different template {generation}" }
        },
        _ => rsx! {
            section {
                header { id: "{class}", "header" }
                for i in 0..(generation % 5) {
                    div { key: "{i}", "item {i}" }
                }
            }
        },
    }
}

#[test]
fn template_swaps_do_not_leave_stale_mappings() {
    let mut doc = make_doc(toggle_app);
    for i in 0..20 {
        step(&mut doc);
        assert_no_stale_mappings(&doc, &format!("step {i}"));
    }
}

/// Pseudo-random mix of templates designed to maximise ElementId and NodeId
/// (slab slot) reuse. Panics with "invalid key" within a handful of steps.
fn fuzz_app() -> Element {
    let generation = generation();
    // Simple LCG for deterministic pseudo-random variety
    let r = generation
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let variant = (r >> 33) % 6;
    let n = (r >> 40) % 7;
    let class = format!("gen-{generation}");
    match variant {
        0 => rsx! {
            div {
                div { class: "{class}", span { "nested {generation}" } }
                "text {generation}"
            }
        },
        1 => rsx! {
            p { "different template {generation}" }
        },
        2 => section_with_list(n),
        3 => rsx! {
            ul {
                for i in 0..n {
                    li { key: "{i}", class: "item-{i}-{generation}",
                        span { "item {i} gen {generation}" }
                    }
                }
            }
        },
        4 => nested_divs(),
        _ => bare_text(),
    }
}

#[test]
fn fuzz_template_swaps_do_not_panic() {
    let mut doc = make_doc(fuzz_app);
    for _ in 0..1000 {
        step(&mut doc);
    }
}

/// A keyed list where items are removed and re-added, forcing ElementId reuse
/// in dioxus-core and NodeId (slab slot) reuse in blitz-dom.
fn list_app() -> Element {
    let generation = generation();
    // Vary the list length up and down so nodes are removed and recreated
    let len = [5usize, 0, 3, 1, 6, 2, 0, 4][generation % 8];
    rsx! {
        ul {
            for i in 0..len {
                li { key: "{i}", class: "item-{i}-{generation}",
                    span { "item {i} gen {generation}" }
                }
            }
        }
        if generation.is_multiple_of(2) {
            footer { "footer {generation}" }
        }
    }
}

#[test]
fn keyed_list_grow_shrink() {
    let mut doc = make_doc(list_app);
    for i in 0..300 {
        step(&mut doc);
        assert_no_stale_mappings(&doc, &format!("step {i}"));
    }
}
