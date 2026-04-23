//! Adversarial tests for rsx-hotreload.
//!
//! These tests probe edge cases around ordering, indexing, and pool matching
//! where the hotreload algorithm is likely to produce wrong outputs.

#![allow(unused)]

use std::collections::HashMap;

use dioxus_core::{
    Template, TemplateAttribute, TemplateNode, VNode,
    internal::{
        FmtSegment, FmtedSegments, HotReloadAttributeValue, HotReloadDynamicAttribute,
        HotReloadDynamicNode, HotReloadLiteral, HotReloadedTemplate, NamedAttribute,
    },
};
use dioxus_core_types::HotReloadingContext;
use dioxus_rsx::CallBody;
use dioxus_rsx_hotreload::HotReloadResult;
use proc_macro2::TokenStream;
use quote::quote;

#[derive(Debug)]
struct Mock;

impl HotReloadingContext for Mock {
    fn map_attribute(
        _element_name_rust: &str,
        _attribute_name_rust: &str,
    ) -> Option<(&'static str, Option<&'static str>)> {
        None
    }

    fn map_element(_element_name_rust: &str) -> Option<(&'static str, Option<&'static str>)> {
        None
    }
}

fn hot_reload_from_tokens(
    old: TokenStream,
    new: TokenStream,
) -> Option<HashMap<usize, HotReloadedTemplate>> {
    let old: CallBody = syn::parse2(old).unwrap();
    let new: CallBody = syn::parse2(new).unwrap();
    let results = HotReloadResult::new::<Mock>(&old.body, &new.body, Default::default())?;
    Some(results.templates)
}

/// Bug A: When two sibling components of DIFFERENT types both have literal props,
/// swapping their positions in the new template produces `component_values` in the
/// wrong order. The runtime indexes `component_values` by the ORIGINAL global
/// literal-pool index (derived from the OLD template's DFS order), but the diff
/// code appends values in NEW-DFS order. The result is a type-mismatch at runtime:
/// a Float slot gets an Int value (or vice-versa), and the component silently
/// renders with Default::default() instead of the new literal.
#[test]
fn swapped_components_different_types_have_correct_literal_order() {
    let old = quote! {
        div {
            Comp1 { a: 1 }
            Comp2 { b: 2.5 }
        }
    };

    let new = quote! {
        div {
            Comp2 { b: 3.5 }
            Comp1 { a: 10 }
        }
    };

    let templates = hot_reload_from_tokens(old, new).expect("should hotreload");
    let template = templates.get(&0).unwrap();

    // The OLD global literal pool layout is:
    //   pool[0] = Comp1.a = Int
    //   pool[1] = Comp2.b = Float
    // So component_values[0] MUST be an Int and component_values[1] MUST be a Float.
    assert_eq!(
        template.component_values,
        &[HotReloadLiteral::Int(10), HotReloadLiteral::Float(3.5)],
        "component_values must be in OLD global literal pool order, not NEW-DFS order"
    );
}

/// Bug B: Non-string literal swap across siblings of the SAME name.
/// When both components have identical prop types, the scoring is ambiguous, and
/// if the diff code picks a swap mapping (new[0] => old[1], new[1] => old[0]),
/// the component_values layout MUST reflect the OLD global literal pool.
#[test]
fn swapped_same_name_components_have_correct_literal_order() {
    let old = quote! {
        div {
            Comp { value: 100 }
            Comp { value: 200 }
        }
    };

    let new = quote! {
        div {
            Comp { value: 200 }
            Comp { value: 100 }
        }
    };

    let templates = hot_reload_from_tokens(old, new).expect("should hotreload");
    let template = templates.get(&0).unwrap();

    // The old compiled code for dyn 0 reads component_values[0].
    // The old compiled code for dyn 1 reads component_values[1].
    //
    // If new_dynamic_nodes[i] = Dynamic(j), slot i in the new tree renders using
    // dyn j's compiled code, which reads component_values[j]. So slot i's value
    // must be placed in component_values[j].
    //
    // new slot 0 wants value 200.
    // new slot 1 wants value 100.
    let HotReloadDynamicNode::Dynamic(slot0_dyn) = template.dynamic_nodes[0] else {
        panic!();
    };
    let HotReloadDynamicNode::Dynamic(slot1_dyn) = template.dynamic_nodes[1] else {
        panic!();
    };

    assert_eq!(
        template.component_values[slot0_dyn],
        HotReloadLiteral::Int(200),
        "new slot 0 wants value 200 but component_values[{}] = {:?}",
        slot0_dyn,
        template.component_values[slot0_dyn]
    );
    assert_eq!(
        template.component_values[slot1_dyn],
        HotReloadLiteral::Int(100),
        "new slot 1 wants value 100 but component_values[{}] = {:?}",
        slot1_dyn,
        template.component_values[slot1_dyn]
    );
}

/// Bug C: Bool/Float/Int type mismatch through swapping.
/// Runtime uses `as_float`/`as_int`/`as_bool` to project the literal pool value to
/// the Rust prop type. If the diff code puts a Bool into a Float slot, the runtime
/// will log an error and fall back to `Default::default()` — a silent correctness
/// bug.
#[test]
fn swapped_components_mixed_literal_types_have_correct_types_per_slot() {
    let old = quote! {
        div {
            A { x: true }
            B { y: 3.14 }
            C { z: 42 }
        }
    };

    let new = quote! {
        div {
            C { z: 43 }
            B { y: 6.28 }
            A { x: false }
        }
    };

    let templates = hot_reload_from_tokens(old, new).expect("should hotreload");
    let template = templates.get(&0).unwrap();

    // OLD pool: [Bool, Float, Int] — exactly that shape.
    // So component_values[0] must be Bool, [1] must be Float, [2] must be Int.
    assert!(
        matches!(template.component_values[0], HotReloadLiteral::Bool(_)),
        "component_values[0] should be Bool (A.x), got {:?}",
        template.component_values[0]
    );
    assert!(
        matches!(template.component_values[1], HotReloadLiteral::Float(_)),
        "component_values[1] should be Float (B.y), got {:?}",
        template.component_values[1]
    );
    assert!(
        matches!(template.component_values[2], HotReloadLiteral::Int(_)),
        "component_values[2] should be Int (C.z), got {:?}",
        template.component_values[2]
    );

    // And the values themselves should be the NEW values, because the swapped
    // positions in the new template get mapped back to the old slots.
    assert_eq!(template.component_values[0], HotReloadLiteral::Bool(false));
    assert_eq!(template.component_values[1], HotReloadLiteral::Float(6.28));
    assert_eq!(template.component_values[2], HotReloadLiteral::Int(43));
}

/// Bug D: Removed-then-reordered components leak component_values.
/// If the new template has fewer components than the old, the surviving components
/// must still end up at their ORIGINAL old global literal-pool indices so the OLD
/// compiled code keeps reading the right slots.
#[test]
fn removed_component_preserves_pool_indices_for_survivors() {
    let old = quote! {
        div {
            A { x: 1 }      // old pool[0] = Int
            B { y: 2.0 }    // old pool[1] = Float
            C { z: true }   // old pool[2] = Bool
        }
    };

    let new = quote! {
        div {
            C { z: false }  // survivor
            A { x: 99 }     // survivor
            // B is removed
        }
    };

    let templates = hot_reload_from_tokens(old, new).expect("should hotreload");
    let template = templates.get(&0).unwrap();

    // The OLD pool is [Int, Float, Bool]. Since A and C survived, their NEW values
    // must land at the SAME old pool slots (0 for A, 2 for C). Slot 1 (B) has no
    // new value; the runtime falls back to Default::default() if the slot is
    // present but wrong-typed, or missing entirely — either way, component_values
    // must be at least length 3 with the surviving values at the right positions.

    // Find mapping to know where each new slot maps.
    let HotReloadDynamicNode::Dynamic(slot0_dyn) = template.dynamic_nodes[0] else {
        panic!();
    };
    let HotReloadDynamicNode::Dynamic(slot1_dyn) = template.dynamic_nodes[1] else {
        panic!();
    };

    // new slot 0 wants C { z: false } — its literal must be reachable from the
    // old C's compiled code, which reads component_values[2]. So we expect slot0
    // to be mapped to old dyn 2 (C), and thus component_values[2] = Bool(false).
    assert_eq!(
        slot0_dyn, 2,
        "first new slot should map to C at old dyn idx 2, got {}",
        slot0_dyn
    );
    assert_eq!(
        slot1_dyn, 0,
        "second new slot should map to A at old dyn idx 0, got {}",
        slot1_dyn
    );

    // Check that both surviving values land in the right component_values slots.
    // There's no requirement on slot 1 (B) since it was removed — it just must
    // not overwrite A's or C's slot.
    let a_pool_idx = 0; // A was first literal in old
    let c_pool_idx = 2; // C was third literal in old
    assert!(
        template.component_values.len() > c_pool_idx,
        "component_values needs at least {} entries, got {}",
        c_pool_idx + 1,
        template.component_values.len()
    );
    assert_eq!(
        template.component_values[a_pool_idx],
        HotReloadLiteral::Int(99),
        "A's slot should have the new int value"
    );
    assert_eq!(
        template.component_values[c_pool_idx],
        HotReloadLiteral::Bool(false),
        "C's slot should have the new bool value"
    );
}

/// Bug F: Three identical-signature components, circular rotation. This is harder
/// because each greedy step has to consider the downstream effects.
#[test]
fn rotated_three_components_have_correct_values() {
    let old = quote! {
        div {
            Comp { x: "a {w}" }
            Comp { x: "b {w}" }
            Comp { x: "c {w}" }
        }
    };

    // Rotate: c -> a -> b -> c
    let new = quote! {
        div {
            Comp { x: "c {w}" }
            Comp { x: "a {w}" }
            Comp { x: "b {w}" }
        }
    };

    let templates = hot_reload_from_tokens(old, new).expect("should hotreload");
    let template = templates.get(&0).unwrap();

    // For each new slot N, its literal should render the expected new label.
    let expected_prefixes = ["c", "a", "b"];
    for (n, expected_prefix) in expected_prefixes.iter().enumerate() {
        let HotReloadDynamicNode::Dynamic(old_idx) = template.dynamic_nodes[n] else {
            panic!("slot {} not Dynamic", n);
        };
        let HotReloadLiteral::Fmted(segs) =
            &template.component_values[old_idx] else {
                panic!("not Fmted");
            };
        // Stringify via debug — we can't reach into private segments field, so
        // compare against the expected FmtedSegments directly instead.
        let expected = FmtedSegments::new(vec![
            FmtSegment::Literal {
                value: Box::leak(format!("{} ", expected_prefix).into_boxed_str()),
            },
            FmtSegment::Dynamic { id: 0 },
        ]);
        assert_eq!(
            segs, &expected,
            "new slot {} (expected prefix '{}') maps to old dyn {} \
             but component_values[{}] = {:?}",
            n, expected_prefix, old_idx, old_idx, segs
        );
    }
}

/// Bug G: The same bug class inside a sub-template (for-loop body). Each nested
/// TemplateBody has its own literal pool; the fix needs to work per-template.
#[test]
fn swapped_components_inside_for_loop_body() {
    let old = quote! {
        div {
            for item in list {
                A { x: 1 }
                B { y: 2.0 }
            }
        }
    };

    let new = quote! {
        div {
            for item in list {
                B { y: 5.0 }
                A { x: 10 }
            }
        }
    };

    let templates = hot_reload_from_tokens(old, new).expect("should hotreload");

    // Find the inner body template — it's the one with the two Dynamic children
    // (the root has 1 dyn node for the for-loop; empty component child bodies have 0).
    let inner = templates
        .values()
        .find(|t| t.dynamic_nodes.len() == 2)
        .expect("should have a for-loop body template with 2 dynamic nodes");

    // OLD pool for the inner body: [A.x=Int, B.y=Float]
    // New swaps positions, so component_values[0] must be Int (A.x=10) and
    // component_values[1] must be Float (B.y=5.0).
    assert_eq!(inner.component_values[0], HotReloadLiteral::Int(10));
    assert_eq!(inner.component_values[1], HotReloadLiteral::Float(5.0));
}

/// Bug H: An if-chain branch body where components are swapped.
#[test]
fn swapped_components_inside_if_chain_branch() {
    let old = quote! {
        if cond {
            A { x: 1 }
            B { y: 2.0 }
        }
    };

    let new = quote! {
        if cond {
            B { y: 7.0 }
            A { x: 11 }
        }
    };

    let templates = hot_reload_from_tokens(old, new).expect("should hotreload");

    let branch = templates
        .values()
        .find(|t| t.dynamic_nodes.len() == 2)
        .expect("should have a branch template with 2 dynamic nodes");

    assert_eq!(branch.component_values[0], HotReloadLiteral::Int(11));
    assert_eq!(branch.component_values[1], HotReloadLiteral::Float(7.0));
}

/// Bug I: When a component at an interior pool slot is removed and the remaining
/// surviving component's prop type doesn't match the removed one, we still must
/// keep the vec at least as long as the highest surviving old pool index AND
/// leave the missing slot with a type-compatible value.
#[test]
fn removed_middle_component_fills_middle_slot_with_compatible_value() {
    let old = quote! {
        div {
            A { x: "{w}" }   // pool[0] = Fmted
            B { y: 2.0 }     // pool[1] = Float
            C { z: 42 }      // pool[2] = Int
        }
    };

    let new = quote! {
        div {
            A { x: "{w}-new" }
            // B removed
            C { z: 100 }
        }
    };

    let templates = hot_reload_from_tokens(old, new).expect("should hotreload");
    let template = templates.get(&0).unwrap();

    // pool[0] surviving → Fmted still
    assert!(matches!(template.component_values[0], HotReloadLiteral::Fmted(_)));
    // pool[1] MISSING → must be Float (type-compatible fallback) so the runtime's
    // coercion doesn't mismatch if somehow read.
    assert!(
        matches!(template.component_values[1], HotReloadLiteral::Float(_)),
        "removed middle slot should be filled with Float fallback, got {:?}",
        template.component_values[1]
    );
    // pool[2] surviving → Int(100)
    assert_eq!(template.component_values[2], HotReloadLiteral::Int(100));
}

/// Bug E: A single component with multiple literal props where props are reordered
/// in the new template should still produce correctly-ordered component_values.
#[test]
fn reordered_props_within_single_component() {
    let old = quote! {
        Comp {
            first: 1,
            second: 2.5,
            third: true,
        }
    };

    let new = quote! {
        Comp {
            third: false,
            first: 10,
            second: 5.0,
        }
    };

    let templates = hot_reload_from_tokens(old, new).expect("should hotreload");
    let template = templates.get(&0).unwrap();

    // OLD pool layout is [first=Int, second=Float, third=Bool]
    assert_eq!(template.component_values[0], HotReloadLiteral::Int(10));
    assert_eq!(template.component_values[1], HotReloadLiteral::Float(5.0));
    assert_eq!(template.component_values[2], HotReloadLiteral::Bool(false));
}

