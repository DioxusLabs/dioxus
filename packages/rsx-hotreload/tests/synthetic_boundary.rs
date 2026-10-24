//! Hot reloading templates that `split_oversized_templates` has broken into
//! [`BodyNode::SyntheticBoundary`] chunks.
//!
//! A template whose root sibling list exceeds the path-bit hard cap is chunk-split into
//! `SyntheticBoundary` wrappers, each of which lowers to its own sub-template *and* occupies a
//! dynamic-node slot in its parent. These tests pin that an edit *inside* such a chunk is picked up
//! by hot reload (rather than being silently dropped) and that the parent template's dynamic-node
//! indices stay aligned with the boundaries.

use dioxus_core::internal::{HotReloadDynamicNode, HotReloadedTemplate};
use dioxus_core_template::TEMPLATE_SLOT_PATH_MAX_PATH_BITS;
use dioxus_core_types::HotReloadingContext;
use dioxus_rsx::{BodyNode, CallBody};
use dioxus_rsx_hotreload::HotReloadResult;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;

#[derive(Debug)]
struct Mock;
impl HotReloadingContext for Mock {
    fn map_attribute(_: &str, _: &str) -> Option<(&'static str, Option<&'static str>)> {
        None
    }
    fn map_element(_: &str) -> Option<(&'static str, Option<&'static str>)> {
        None
    }
}

/// A root sibling count guaranteed to exceed the path-bit hard cap and force a chunk split.
///
/// A root at index `i` carries `1 + i` path bits, so a list of `n` roots overflows once
/// `1 + (n - 1) > LIMIT`, i.e. `n > LIMIT`. We add a small margin and stay well under the op /
/// dynamic-node caps, so path bits are the only limit that trips. Deriving this from the live
/// constant keeps the test correct if the limit is retuned.
fn oversized_root_count() -> usize {
    TEMPLATE_SLOT_PATH_MAX_PATH_BITS + 4
}

/// Build a `rsx!` body with `count` root-level `div { "<label> i: {x}" }` siblings, except that the
/// node at `changed` (if any) gets a `CHANGED` literal so we can find the edit in the output.
fn wide_body(count: usize, label: &str, changed: Option<usize>) -> TokenStream {
    let mut out = TokenStream::new();
    for i in 0..count {
        let text = if changed == Some(i) {
            format!("CHANGED {i}: {{x}}")
        } else {
            format!("{label} {i}: {{x}}")
        };
        out.extend(quote! { div { #text } });
    }
    out
}

/// Count `SyntheticBoundary` nodes anywhere in the body tree.
fn count_synthetic(nodes: &[BodyNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            BodyNode::SyntheticBoundary(body) => 1 + count_synthetic(&body.roots),
            BodyNode::Element(el) => count_synthetic(&el.children),
            BodyNode::Component(c) => count_synthetic(&c.children.roots),
            BodyNode::ForLoop(f) => count_synthetic(&f.body.roots),
            _ => 0,
        })
        .sum()
}

/// Does any produced template contain a formatted dynamic node whose literal segments include
/// `needle`? Used to prove an edit actually made it into the hot reload payload. `FmtedSegments`
/// keeps its segments private, so we inspect its `Debug` form, which renders each literal value.
fn payload_contains(templates: &HashMap<usize, HotReloadedTemplate>, needle: &str) -> bool {
    templates.values().any(|template| {
        template.dynamic_nodes.iter().any(|node| {
            matches!(node, HotReloadDynamicNode::Formatted(_))
                && format!("{node:?}").contains(needle)
        })
    })
}

fn parse(tokens: TokenStream) -> CallBody {
    syn::parse2(tokens).unwrap()
}

/// An oversized root list is chunk-split into synthetic boundaries that sit at the root.
#[test]
fn wide_template_produces_synthetic_boundaries() {
    let body = parse(wide_body(oversized_root_count(), "x", None));
    // The top-level list is halved once, so the roots become two boundaries.
    assert_eq!(body.body().roots.len(), 2);
    assert!(
        body.body()
            .roots
            .iter()
            .all(|n| matches!(n, BodyNode::SyntheticBoundary(_)))
    );
    assert!(count_synthetic(&body.body().roots) >= 2);
}

/// Editing a literal *inside* a synthetic boundary hot reloads the change instead of dropping it,
/// and keeps the parent template's dynamic-node slots aligned with the boundaries.
#[test]
fn edit_inside_synthetic_boundary_is_hot_reloaded() {
    let count = oversized_root_count();
    let old = parse(wide_body(count, "old", None));
    let new = parse(wide_body(count, "old", Some(count / 2)));

    // Sanity: the templates really are oversized and split into root-level boundaries.
    let root_boundaries = old.body().roots.len();
    assert!(root_boundaries >= 2);
    assert!(
        old.body()
            .roots
            .iter()
            .all(|n| matches!(n, BodyNode::SyntheticBoundary(_)))
    );

    let result = HotReloadResult::new::<Mock>(old.body(), new.body(), Default::default())
        .expect("editing a literal inside an oversized template should hot reload, not bail");

    // The edit must be present in the payload — this is the regression guard against the change
    // being silently dropped.
    assert!(
        payload_contains(&result.templates, &format!("CHANGED {}", count / 2)),
        "the edited literal must appear in some produced sub-template"
    );

    // The root template must carry one dynamic-node slot per root-level boundary, each pointing at a
    // distinct old slot. A skewed/empty list here is the original A3 bug.
    let root = &result.templates[&0];
    assert_eq!(root.dynamic_nodes.len(), root_boundaries);
    let mut targets: Vec<usize> = root
        .dynamic_nodes
        .iter()
        .map(|node| match node {
            HotReloadDynamicNode::Dynamic(index) => *index,
            other => panic!("expected boundary slots to be Dynamic, got {other:?}"),
        })
        .collect();
    targets.sort();
    targets.dedup();
    assert_eq!(
        targets.len(),
        root_boundaries,
        "each boundary must map to a distinct old dynamic-node slot"
    );
}

/// An unchanged oversized template still round-trips through hot reload (every boundary matches its
/// counterpart) and surfaces all of its sub-templates.
#[test]
fn unchanged_synthetic_boundary_round_trips() {
    let body = parse(wide_body(oversized_root_count(), "x", None));
    let result = HotReloadResult::new::<Mock>(body.body(), body.body(), Default::default())
        .expect("an oversized template should hot reload against itself");

    // Root + one sub-template per boundary.
    assert_eq!(
        result.templates.len(),
        1 + count_synthetic(&body.body().roots)
    );
}

/// An edit inside a boundary that *cannot* be hot reloaded (a brand-new dynamic expression with no
/// match in the old build) bails to `None`, so the CLI falls back to a full rebuild rather than
/// applying a partial template.
#[test]
fn unhotreloadable_edit_inside_boundary_bails() {
    let count = oversized_root_count();
    let old = parse(wide_body(count, "old", None));

    // Replace one node's body with a fresh dynamic node (a raw expr) the old build never had.
    let mut new_inner = TokenStream::new();
    for i in 0..count {
        if i == count / 2 {
            new_inner.extend(quote! { div { {some_brand_new_signal} } });
        } else {
            let text = format!("old {i}: {{x}}");
            new_inner.extend(quote! { div { #text } });
        }
    }
    let new = parse(new_inner);

    assert!(
        HotReloadResult::new::<Mock>(old.body(), new.body(), Default::default()).is_none(),
        "an un-hot-reloadable edit inside a boundary must bail so the CLI does a full rebuild"
    );
}
