//! Reusable Dioxus VirtualDom fuzzing harness.
//!
//! The `cargo-fuzz` target feeds encoded [`FuzzCase`] values into this crate.
//! LibFuzzer owns coverage guidance and corpus management; this crate owns the
//! structured operation stream and renderer oracle.
#![deny(unsafe_code)]

mod cache;
mod context;
mod diagnostics;
mod event;
mod harness;
mod lifecycle;
mod model;
mod ops;
mod reducer;
mod vdom;

use diagnostics::panic_message;
use harness::{Harness, apply_step, print_ssr_diff_trace};
use model::{
    AttrSpec, AttrValueSpec, DynamicKind, DynamicSpec, FragmentKeyMode, Model, SuspenseMode,
    TemplateAttrSpec, TemplateNodeKind, TemplateNodeSpec, VNodeSpec, WakeMutationSpec,
};
use mutatis::{Candidates, Generate, Mutate, Result as MutatisResult, Session};
use ops::{EventBehaviorSpec, FragmentEdit, ListEdit, Op, TemplateEdit};
pub use reducer::ReductionOptions;
use reducer::{random_multistep_shrink_case, simplified_ops};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    panic::{self, AssertUnwindSafe},
};

const MAX_STEPS: usize = 512;
const PRIMITIVE_MUTATION_COUNT: u32 = 20;

/// Fold every attribute name into a 16-slot pool so static and dynamic
/// attributes on the same element collide on the same `(name, namespace)`
/// key often enough for `remove_attribute_or_write_fallback` to fire.
pub(crate) const ATTR_NAME_POOL_MASK: u8 = 0x0F;

pub struct FuzzCase {
    ops: Vec<Op>,
}

impl FuzzCase {
    pub(crate) fn new(mut ops: Vec<Op>) -> Self {
        ops.truncate(MAX_STEPS);
        Self { ops }
    }

    fn normalize(&mut self) {
        self.ops.truncate(MAX_STEPS);
    }

    fn clone_case(&self) -> Self {
        Self {
            ops: self.ops.clone(),
        }
    }
}

impl Default for FuzzCase {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[derive(Clone, Debug, Default)]
struct FuzzCaseMutator;

impl Mutate<FuzzCase> for FuzzCaseMutator {
    fn mutate(
        &mut self,
        candidates: &mut Candidates<'_>,
        case: &mut FuzzCase,
    ) -> MutatisResult<()> {
        if candidates.shrink() {
            return shrink_case(candidates, case);
        }

        if case.ops.len() < MAX_STEPS {
            candidates.mutation(|context| {
                let index = context.rng().gen_index(case.ops.len() + 1).unwrap();
                let mut op_mutator = mutatis::mutators::default::<Op>();
                let op = op_mutator.generate(context)?;
                case.ops.insert(index, op);
                Ok(())
            })?;
        }

        candidates.mutation_group(PRIMITIVE_MUTATION_COUNT, |context, which| {
            splice_primitive_op(context, case, which);
            Ok(())
        })?;

        if !case.ops.is_empty() {
            candidates.mutation(|context| {
                let index = context.rng().gen_index(case.ops.len()).unwrap();
                case.ops.remove(index);
                Ok(())
            })?;
        }

        if case.ops.len() >= 2 {
            candidates.mutation(|context| {
                let left = context.rng().gen_index(case.ops.len()).unwrap();
                let right = context.rng().gen_index(case.ops.len()).unwrap();
                case.ops.swap(left, right);
                Ok(())
            })?;
        }

        let mut op_mutator = mutatis::mutators::default::<Op>();
        for op in &mut case.ops {
            op_mutator.mutate(candidates, op)?;
        }

        case.normalize();

        Ok(())
    }
}

pub fn mutate_case(
    case: &mut FuzzCase,
    seed: u32,
    shrink: bool,
    additional_mutations: usize,
) -> bool {
    let mut session = Session::new().seed(seed.into()).shrink(shrink);
    let mut mutator = FuzzCaseMutator;

    if session.mutate_with(&mut mutator, case).is_err() {
        return false;
    }

    for _ in 0..additional_mutations {
        if session.mutate_with(&mut mutator, case).is_err() {
            break;
        }
    }

    case.normalize();
    true
}

fn replay_model_prefix(ops: &[Op], len: usize) -> Model {
    let mut model = Model::initial();
    for op in ops.iter().take(len) {
        ops::apply_strategy_op_to_model(&mut model, op);
    }
    model
}

fn splice_primitive_op(context: &mut mutatis::Context, case: &mut FuzzCase, which: u32) {
    let index = context.rng().gen_index(case.ops.len() + 1).unwrap();
    let model = replay_model_prefix(&case.ops, index);
    let selector = context.rng().gen_u8();
    let value = context.rng().gen_u8();
    let ops = biased_primitive_op_sequence(&model, which, selector, value);
    for (offset, op) in ops.into_iter().enumerate() {
        if case.ops.len() < MAX_STEPS {
            case.ops.insert(index + offset, op);
        } else {
            let replace = (index + offset).min(case.ops.len() - 1);
            case.ops[replace] = op;
        }
    }
}

fn biased_primitive_op_sequence(model: &Model, which: u32, selector: u8, value: u8) -> Vec<Op> {
    if which == 19 {
        if let Some(ops) = collision_aliasing_sequence(model, selector, value) {
            return ops;
        }
    }
    vec![biased_primitive_op(model, which, selector, value)]
}

fn fragment_insert_key(fragment: FragmentShape, value: u8) -> Option<u8> {
    fragment
        .keyed
        .then_some(value.wrapping_add(fragment.len.min(u8::MAX as usize) as u8))
}

#[cfg(test)]
fn dynamic_attribute_static_fallback_recipe() -> Vec<Op> {
    vec![
        Op::template(
            0,
            TemplateEdit::Attrs {
                element: 0,
                edit: ListEdit::Insert {
                    index: 0,
                    item: TemplateAttrSpec::Static {
                        name: 33,
                        value: 129,
                        namespace: None,
                    },
                },
            },
        ),
        Op::template(
            0,
            TemplateEdit::Attrs {
                element: 0,
                edit: ListEdit::Insert {
                    index: 1,
                    item: TemplateAttrSpec::Dynamic(Vec::new()),
                },
            },
        ),
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 33,
                    namespace: None,
                    value: AttrValueSpec::Text(2),
                    volatile: false,
                },
            },
        ),
        Op::Rerender,
        Op::dynamic_attrs(0, 0, ListEdit::Remove { index: 0 }),
        Op::Rerender,
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 33,
                    namespace: None,
                    value: AttrValueSpec::Text(1),
                    volatile: false,
                },
            },
        ),
        Op::Rerender,
        Op::dynamic_attrs(0, 0, ListEdit::Remove { index: 0 }),
        Op::Rerender,
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 33,
                    namespace: None,
                    value: AttrValueSpec::Int(3),
                    volatile: false,
                },
            },
        ),
        Op::Rerender,
        Op::dynamic_attrs(0, 0, ListEdit::Remove { index: 0 }),
        Op::dynamic_attrs(
            0,
            0,
            ListEdit::Insert {
                index: 0,
                item: AttrSpec {
                    name: 33,
                    namespace: None,
                    value: AttrValueSpec::Bool(true),
                    volatile: false,
                },
            },
        ),
        Op::Rerender,
    ]
}

fn biased_primitive_op(model: &Model, which: u32, selector: u8, value: u8) -> Op {
    let facts = ModelFacts::new(model);
    let vnode = facts.select_focus_vnode(selector, value);
    let node = facts.select_node(vnode, value);
    let element = facts.select_element(vnode, value);
    match which {
        0 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::SetNode {
                node,
                kind: biased_template_node_kind(value),
            },
        ),
        1 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::Roots {
                edit: ListEdit::Insert {
                    index: biased_index(value, facts.root_count(vnode)),
                    item: biased_template_node_kind(value),
                },
            },
        ),
        2 => Op::template(
            vnode,
            TemplateEdit::Roots {
                edit: remove_or_move_list_edit(facts.root_count(vnode), selector, value),
            },
        ),
        3 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::Children {
                element,
                edit: ListEdit::Insert {
                    index: biased_index(value, facts.child_count(vnode, element)),
                    item: biased_template_node_kind(value),
                },
            },
        ),
        4 => Op::template(
            vnode,
            TemplateEdit::Children {
                element,
                edit: remove_or_move_list_edit(facts.child_count(vnode, element), selector, value),
            },
        ),
        5 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::Attrs {
                element,
                edit: ListEdit::Insert {
                    index: biased_index(value, facts.template_attr_count(vnode, element)),
                    item: biased_template_attr(value),
                },
            },
        ),
        6 => Op::template(
            vnode,
            TemplateEdit::Attrs {
                element,
                edit: remove_or_move_list_edit(
                    facts.template_attr_count(vnode, element),
                    selector,
                    value,
                ),
            },
        ),
        7 => dynamic_node_op(&facts, vnode, selector, biased_fragment_dynamic_kind(value)),
        8 => dynamic_node_op(&facts, vnode, selector, biased_leaf_dynamic_kind(value)),
        9 => dynamic_node_op(
            &facts,
            vnode,
            selector,
            if value & 1 == 0 {
                DynamicKind::ComponentA
            } else {
                DynamicKind::ComponentB
            },
        ),
        10 if facts.has_dynamic_nodes() => {
            let fragment = facts
                .select_fragment(selector)
                .unwrap_or_else(|| facts.fragment_prerequisite(selector));
            Op::fragment(
                fragment.vnode,
                fragment.node,
                FragmentEdit::KeyMode(biased_fragment_key_mode(value)),
            )
        }
        10 => dynamic_node_op(&facts, vnode, selector, biased_fragment_dynamic_kind(value)),
        11 if facts.has_dynamic_nodes() => {
            edit_fragment_children_op(&facts, model.can_grow(), selector, value)
        }
        11 => dynamic_node_op(&facts, vnode, selector, biased_fragment_dynamic_kind(value)),
        12 => edit_dynamic_attrs_op(&facts, model.can_grow(), vnode, element, selector, value),
        13 if facts.has_suspense() => {
            Op::suspense(facts.select_suspense(selector), biased_suspense_mode(value))
        }
        13 => dynamic_node_op(
            &facts,
            vnode,
            selector,
            suspense_kind(biased_suspense_mode(value)),
        ),
        14 if facts.has_suspense() => {
            Op::suspense_wake_mutation(facts.select_suspense(selector), biased_wake_mutation(value))
        }
        14 => ready_suspense_node_op(&facts, vnode, selector),
        15 if facts.has_suspense() => Op::wake_suspense(facts.select_suspense(selector)),
        15 => ready_suspense_node_op(&facts, vnode, selector),
        16 => Op::fire_event(
            selector,
            if value & 1 == 0 {
                EventBehaviorSpec::Noop
            } else {
                EventBehaviorSpec::DispatchNestedEvent { target: selector }
            },
        ),
        17 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::SetNode {
                node,
                kind: TemplateNodeKind::Element {
                    tag: value,
                    namespace: (selector & 1 == 0).then_some(selector),
                },
            },
        ),
        18 => Op::Rerender,
        // Note: `which == 19` is handled specially by
        // `biased_primitive_op_sequence` (it can emit a paired alias-then-
        // remove sequence). If the splice path falls through to this arm
        // because the model has no dynamic attribute to alias, fall back to
        // a SetNode op so we still produce something useful.
        19 => Op::template(
            vnode,
            TemplateEdit::SetNode {
                node,
                kind: TemplateNodeKind::Dynamic(biased_leaf_dynamic_kind(value)),
            },
        ),
        _ => Op::template(
            vnode,
            TemplateEdit::SetNode {
                node,
                kind: TemplateNodeKind::Dynamic(biased_leaf_dynamic_kind(value)),
            },
        ),
    }
}

/// Build the alias-then-remove sequence that drives
/// `diff_attributes::remove_attribute_or_write_fallback`.
///
/// Step 1 inserts a *static* template attribute on the element with the same
/// resolved name as one of its existing dynamic attributes. Step 2 removes
/// the dynamic side via a `Rerender` so the diff can compare the two
/// renders, then a `DynamicAttrs::Remove` op that disposes of the colliding
/// dynamic attribute. After the next `Rerender`, the diff sees:
///   old: dynamic at K  /  new: dynamic gone, static at K still on template
/// → `remove_attribute_or_write_fallback` falls back to the static value.
fn collision_aliasing_sequence(model: &Model, selector: u8, value: u8) -> Option<Vec<Op>> {
    let mut candidates: Vec<CollisionCandidate> = Vec::new();
    collect_collision_candidates(&model.root, 0, &mut 0u8, &mut candidates);
    let pick = *candidates.get(selector as usize % candidates.len().max(1))?;
    let alias = Op::template(
        pick.vnode,
        TemplateEdit::Attrs {
            element: pick.element,
            edit: ListEdit::Insert {
                index: biased_index(value, pick.element_attr_count),
                item: TemplateAttrSpec::Static {
                    // Copy the dynamic attribute's name byte verbatim. The
                    // candidate collector filters to non-listener bytes
                    // with high bit clear, so this resolves to
                    // `attr_name(name)` == the dynamic side's
                    // `attr_name(name)` — a real key collision.
                    name: pick.dynamic_name,
                    value: value.wrapping_add(1),
                    namespace: None,
                },
            },
        },
    );
    // Schedule the dynamic drop right after the alias. The fuzz target
    // already injects `Rerender` ops on its own; chaining alias+drop without
    // explicit rerenders keeps the case short so other diff paths still get
    // op budget.
    let drop_dynamic =
        Op::dynamic_attrs(pick.vnode, pick.dynamic_slot, ListEdit::Remove { index: 0 });
    Some(vec![alias, drop_dynamic])
}

#[derive(Clone, Copy)]
struct CollisionCandidate {
    vnode: u8,
    element: u8,
    element_attr_count: usize,
    dynamic_slot: u8,
    dynamic_name: u8,
}

fn collect_collision_candidates(
    vnode: &VNodeSpec,
    vnode_index_hint: u8,
    next_vnode_index: &mut u8,
    out: &mut Vec<CollisionCandidate>,
) {
    let vnode_index = vnode_index_hint;
    // Track the depth-first element index within this vnode and the global
    // dynamic-attr slot counter to match `ModelFacts::select_element` and the
    // `attr` numbering consumed by `selected_dynamic_attr_mut`.
    let mut element_index: u8 = 0;
    let mut dynamic_slot: u8 = 0;
    walk_template_for_collisions(
        vnode_index,
        &vnode.template.roots,
        &mut element_index,
        &mut dynamic_slot,
        out,
    );

    // Recurse into nested vnodes produced by fragments / suspense children so
    // we can also target attributes inside those subtrees. The numbering
    // matches `ModelFacts::collect_vnode`'s pre-order traversal.
    walk_dynamic_for_nested_vnodes(&vnode.template.roots, next_vnode_index, out);
}

fn walk_template_for_collisions(
    vnode: u8,
    nodes: &[TemplateNodeSpec],
    element_index: &mut u8,
    dynamic_slot: &mut u8,
    out: &mut Vec<CollisionCandidate>,
) {
    for node in nodes {
        if let TemplateNodeSpec::Element {
            attrs, children, ..
        } = node
        {
            let element = *element_index;
            *element_index = element_index.saturating_add(1);

            for attr in attrs {
                if let TemplateAttrSpec::Dynamic(dynamic_attrs) = attr {
                    let slot = *dynamic_slot;
                    *dynamic_slot = dynamic_slot.saturating_add(1);
                    for dyn_attr in dynamic_attrs {
                        // Skip listeners (their name space is disjoint from
                        // static attribute names) and skip any byte whose
                        // high bit is set, since `dynamic_attr_name` routes
                        // those through the listener naming path regardless
                        // of the AttrValueSpec variant.
                        if matches!(dyn_attr.value, AttrValueSpec::Listener)
                            || dyn_attr.name & 0x80 != 0
                        {
                            continue;
                        }
                        out.push(CollisionCandidate {
                            vnode,
                            element,
                            element_attr_count: attrs.len(),
                            dynamic_slot: slot,
                            dynamic_name: dyn_attr.name,
                        });
                    }
                }
            }

            walk_template_for_collisions(vnode, children, element_index, dynamic_slot, out);
        }
    }
}

fn walk_dynamic_for_nested_vnodes(
    nodes: &[TemplateNodeSpec],
    next_vnode_index: &mut u8,
    out: &mut Vec<CollisionCandidate>,
) {
    for node in nodes {
        match node {
            TemplateNodeSpec::Element { children, .. } => {
                walk_dynamic_for_nested_vnodes(children, next_vnode_index, out);
            }
            TemplateNodeSpec::Dynamic(DynamicSpec::Fragment(children)) => {
                for child in children {
                    *next_vnode_index = next_vnode_index.saturating_add(1);
                    let child_index = *next_vnode_index;
                    collect_collision_candidates(child, child_index, next_vnode_index, out);
                }
            }
            _ => {}
        }
    }
}

fn dynamic_node_op(facts: &ModelFacts, vnode: u8, selector: u8, kind: DynamicKind) -> Op {
    Op::dynamic(vnode, facts.select_dynamic_node(vnode, selector), kind)
}

fn ready_suspense_node_op(facts: &ModelFacts, vnode: u8, selector: u8) -> Op {
    dynamic_node_op(
        facts,
        vnode,
        selector,
        suspense_kind(SuspenseMode::Ready { wake_after: 0 }),
    )
}

fn edit_fragment_children_op(facts: &ModelFacts, can_grow: bool, selector: u8, value: u8) -> Op {
    let fragment = facts
        .select_fragment(selector)
        .unwrap_or_else(|| facts.fragment_prerequisite(selector));
    let edit = match value % 3 {
        0 if can_grow => ListEdit::Insert {
            index: biased_index(value, fragment.len),
            item: fragment_insert_key(fragment, value),
        },
        1 if fragment.len > 0 => ListEdit::Remove {
            index: biased_existing_index(value, fragment.len),
        },
        2 if fragment.len >= 2 => ListEdit::Move {
            from: biased_existing_index(selector, fragment.len),
            to: biased_index(value, fragment.len),
        },
        _ if can_grow => ListEdit::Insert {
            index: 0,
            item: fragment_insert_key(fragment, value),
        },
        _ => ListEdit::Remove { index: 0 },
    };

    Op::fragment(fragment.vnode, fragment.node, FragmentEdit::Children(edit))
}

fn edit_dynamic_attrs_op(
    facts: &ModelFacts,
    can_grow: bool,
    vnode: u8,
    element: u8,
    selector: u8,
    value: u8,
) -> Op {
    let Some(attr) = facts.select_attr_slot(selector) else {
        return prerequisite_dynamic_attr_op(facts, vnode, element, value);
    };

    let edit = match value % 3 {
        0 => ListEdit::Insert {
            index: biased_index(value, attr.len),
            item: biased_attr(value),
        },
        1 if attr.len > 0 => ListEdit::Remove {
            index: biased_existing_index(value, attr.len),
        },
        2 if attr.len >= 2 => ListEdit::Move {
            from: biased_existing_index(selector, attr.len),
            to: biased_index(value, attr.len),
        },
        _ if can_grow => ListEdit::Insert {
            index: biased_index(value, attr.len),
            item: biased_attr(value),
        },
        _ => ListEdit::Remove { index: 0 },
    };

    Op::dynamic_attrs(attr.vnode, attr.slot, edit)
}

fn prerequisite_dynamic_attr_op(facts: &ModelFacts, vnode: u8, element: u8, value: u8) -> Op {
    Op::template(
        vnode,
        TemplateEdit::Attrs {
            element,
            edit: ListEdit::Insert {
                index: biased_index(value, facts.template_attr_count(vnode, element)),
                item: TemplateAttrSpec::Dynamic(vec![biased_attr(value)]),
            },
        },
    )
}

#[derive(Clone, Copy)]
struct FragmentShape {
    vnode: u8,
    node: u8,
    len: usize,
    keyed: bool,
}

#[derive(Clone, Copy)]
struct AttrShape {
    vnode: u8,
    slot: u8,
    len: usize,
}

#[derive(Default)]
struct VNodeShape {
    roots: usize,
    nodes: usize,
    elements: Vec<ElementShape>,
    dynamic_nodes: Vec<u8>,
}

#[derive(Clone)]
struct ElementShape {
    children: usize,
    attrs: usize,
}

#[derive(Default)]
struct ModelFacts {
    vnodes: Vec<VNodeShape>,
    fragments: Vec<FragmentShape>,
    attrs: Vec<AttrShape>,
    suspense_child_vnodes: Vec<u8>,
    suspense_count: usize,
}

impl ModelFacts {
    fn new(model: &Model) -> Self {
        let mut facts = Self::default();
        facts.collect_vnode(&model.root, None);
        facts
    }

    fn collect_vnode(&mut self, vnode: &VNodeSpec, suspense: Option<u8>) -> u8 {
        let vnode_index = self.vnodes.len() as u8;
        let mut elements = Vec::new();
        let mut attr_slot = 0;
        self.collect_template_elements_and_attrs(
            vnode_index,
            &vnode.template.roots,
            &mut attr_slot,
            &mut elements,
        );

        self.vnodes.push(VNodeShape {
            roots: vnode.template.roots.len(),
            nodes: vnode.template.node_paths().len(),
            elements,
            dynamic_nodes: vnode
                .template
                .node_paths()
                .into_iter()
                .enumerate()
                .filter_map(|(index, path)| {
                    matches!(
                        template_node_at(&vnode.template.roots, &path),
                        Some(TemplateNodeSpec::Dynamic(_))
                    )
                    .then_some(index.min(u8::MAX as usize) as u8)
                })
                .collect(),
        });

        let mut dynamic_slot = 0;
        self.collect_dynamic_nodes(
            vnode_index,
            &vnode.template.roots,
            suspense,
            &mut dynamic_slot,
        );

        vnode_index
    }

    fn collect_template_elements_and_attrs(
        &mut self,
        vnode: u8,
        nodes: &[TemplateNodeSpec],
        slot: &mut usize,
        elements: &mut Vec<ElementShape>,
    ) {
        for node in nodes {
            let TemplateNodeSpec::Element {
                attrs, children, ..
            } = node
            else {
                continue;
            };

            for attr in attrs {
                if let TemplateAttrSpec::Dynamic(attrs) = attr {
                    let dynamic_slot = (*slot).min(u8::MAX as usize) as u8;
                    self.attrs.push(AttrShape {
                        vnode,
                        slot: dynamic_slot,
                        len: attrs.len(),
                    });
                    *slot += 1;
                }
            }

            elements.push(ElementShape {
                children: children.len(),
                attrs: attrs.len(),
            });

            self.collect_template_elements_and_attrs(vnode, children, slot, elements);
        }
    }

    fn collect_dynamic_nodes(
        &mut self,
        vnode: u8,
        nodes: &[TemplateNodeSpec],
        suspense: Option<u8>,
        slot: &mut usize,
    ) {
        for node in nodes {
            match node {
                TemplateNodeSpec::Element { children, .. } => {
                    self.collect_dynamic_nodes(vnode, children, suspense, slot);
                }
                TemplateNodeSpec::Text(_) => {}
                TemplateNodeSpec::Dynamic(dynamic) => {
                    let current_slot = (*slot).min(u8::MAX as usize) as u8;
                    *slot += 1;
                    match dynamic {
                        DynamicSpec::Fragment(children) => {
                            for child in children {
                                self.collect_vnode(child, suspense);
                            }
                            self.fragments.push(FragmentShape {
                                vnode,
                                node: current_slot,
                                len: children.len(),
                                keyed: children.first().and_then(|child| child.key).is_some(),
                            });
                        }
                        DynamicSpec::ComponentA(component) | DynamicSpec::ComponentB(component) => {
                            self.collect_vnode(&component.child, suspense);
                        }
                        DynamicSpec::Suspense(suspense) => {
                            let suspense_index = self.suspense_count.min(u8::MAX as usize) as u8;
                            self.suspense_count += 1;
                            let child = self.collect_vnode(&suspense.child, Some(suspense_index));
                            self.suspense_child_vnodes.push(child);
                        }
                        DynamicSpec::Portal(child) => {
                            self.collect_vnode(child, suspense);
                        }
                        DynamicSpec::Empty | DynamicSpec::Text(_) | DynamicSpec::Placeholder => {}
                    }
                }
            }
        }
    }

    fn select_vnode(&self, selector: u8) -> u8 {
        select_bounded(selector, self.vnodes.len())
    }

    fn select_focus_vnode(&self, selector: u8, value: u8) -> u8 {
        match value % 4 {
            0 if !self.suspense_child_vnodes.is_empty() => {
                self.suspense_child_vnodes[selector as usize % self.suspense_child_vnodes.len()]
            }
            1 if self.vnodes.len() > 1 => (1 + select_bounded(selector, self.vnodes.len() - 1)
                as usize)
                .min(u8::MAX as usize) as u8,
            _ => self.select_vnode(selector),
        }
    }

    fn select_node(&self, vnode: u8, selector: u8) -> u8 {
        select_bounded(selector, self.vnodes[vnode as usize].nodes)
    }

    fn root_count(&self, vnode: u8) -> usize {
        self.vnodes[vnode as usize].roots
    }

    fn select_element(&self, vnode: u8, selector: u8) -> u8 {
        select_bounded(selector, self.vnodes[vnode as usize].elements.len())
    }

    fn child_count(&self, vnode: u8, element: u8) -> usize {
        self.vnodes[vnode as usize]
            .elements
            .get(element as usize)
            .map(|element| element.children)
            .unwrap_or(0)
    }

    fn template_attr_count(&self, vnode: u8, element: u8) -> usize {
        self.vnodes[vnode as usize]
            .elements
            .get(element as usize)
            .map(|element| element.attrs)
            .unwrap_or(0)
    }

    fn select_dynamic_node(&self, vnode: u8, selector: u8) -> u8 {
        let vnode_shape = &self.vnodes[vnode as usize];
        vnode_shape
            .dynamic_nodes
            .get(selector as usize % vnode_shape.dynamic_nodes.len().max(1))
            .copied()
            .unwrap_or_else(|| self.select_node(vnode, selector))
    }

    fn has_dynamic_nodes(&self) -> bool {
        self.vnodes
            .iter()
            .any(|vnode| !vnode.dynamic_nodes.is_empty())
    }

    fn select_fragment(&self, selector: u8) -> Option<FragmentShape> {
        self.fragments
            .get(selector as usize % self.fragments.len().max(1))
            .copied()
    }

    fn fragment_prerequisite(&self, selector: u8) -> FragmentShape {
        let vnode = self.select_vnode(selector);
        let vnode_shape = &self.vnodes[vnode as usize];
        FragmentShape {
            vnode,
            node: select_bounded(selector, vnode_shape.dynamic_nodes.len()),
            len: 0,
            keyed: false,
        }
    }

    fn select_attr_slot(&self, selector: u8) -> Option<AttrShape> {
        self.attrs
            .get(selector as usize % self.attrs.len().max(1))
            .copied()
    }

    fn select_suspense(&self, selector: u8) -> u8 {
        select_bounded(selector, self.suspense_count)
    }

    fn has_suspense(&self) -> bool {
        self.suspense_count > 0
    }
}

fn template_node_at<'a>(
    roots: &'a [TemplateNodeSpec],
    path: &[usize],
) -> Option<&'a TemplateNodeSpec> {
    let (&root, rest) = path.split_first()?;
    let mut node = roots.get(root)?;
    for index in rest {
        let TemplateNodeSpec::Element { children, .. } = node else {
            return None;
        };
        node = children.get(*index)?;
    }
    Some(node)
}

fn select_bounded(selector: u8, len: usize) -> u8 {
    if len == 0 {
        0
    } else {
        (selector as usize % len).min(u8::MAX as usize) as u8
    }
}

fn biased_index(selector: u8, len: usize) -> u8 {
    match selector % 5 {
        0 => 0,
        1 => (len / 2).min(u8::MAX as usize) as u8,
        2 => len.min(u8::MAX as usize) as u8,
        3 => len.saturating_sub(1).min(u8::MAX as usize) as u8,
        _ => selector,
    }
}

fn biased_existing_index(selector: u8, len: usize) -> u8 {
    biased_index(selector, len.saturating_sub(1))
}

fn remove_or_move_list_edit<T>(len: usize, selector: u8, value: u8) -> ListEdit<T> {
    if selector & 1 == 0 {
        ListEdit::Remove {
            index: biased_existing_index(value, len),
        }
    } else {
        ListEdit::Move {
            from: biased_existing_index(selector, len),
            to: biased_index(value, len),
        }
    }
}

fn biased_template_node_kind(value: u8) -> TemplateNodeKind {
    match value % 3 {
        0 => TemplateNodeKind::Dynamic(biased_dynamic_kind(value)),
        1 => TemplateNodeKind::Text(value),
        _ => TemplateNodeKind::Element {
            tag: value,
            namespace: (value & 1 == 0).then_some(value.wrapping_add(1)),
        },
    }
}

fn biased_template_attr(value: u8) -> TemplateAttrSpec {
    if value & 1 == 0 {
        TemplateAttrSpec::Dynamic(vec![biased_attr(value)])
    } else {
        TemplateAttrSpec::Static {
            // Mask the name into the shared pool so this static attribute
            // can collide with a dynamic attribute on the same element and
            // exercise `remove_attribute_or_write_fallback`.
            name: value & ATTR_NAME_POOL_MASK,
            value: value.wrapping_add(1),
            namespace: (value & 2 == 0).then_some(value.wrapping_add(2)),
        }
    }
}

fn biased_dynamic_kind(value: u8) -> DynamicKind {
    match value % 6 {
        0 => biased_leaf_dynamic_kind(value),
        1 => biased_fragment_dynamic_kind(value),
        2 => DynamicKind::ComponentA,
        3 => DynamicKind::ComponentB,
        4 => suspense_kind(biased_suspense_mode(value)),
        _ => DynamicKind::Placeholder,
    }
}

fn biased_leaf_dynamic_kind(value: u8) -> DynamicKind {
    match value % 3 {
        0 => DynamicKind::Text(value),
        1 => DynamicKind::Placeholder,
        _ => DynamicKind::Empty,
    }
}

fn suspense_kind(mode: SuspenseMode) -> DynamicKind {
    DynamicKind::Suspense { mode }
}

fn biased_fragment_dynamic_kind(value: u8) -> DynamicKind {
    DynamicKind::Fragment {
        children: (value % 3).saturating_add(1),
        key_base: (value & 4 != 0).then_some(value),
    }
}

fn biased_suspense_mode(value: u8) -> SuspenseMode {
    match value % 3 {
        0 => SuspenseMode::Resolved,
        1 => SuspenseMode::Pending,
        _ => SuspenseMode::Ready {
            wake_after: value / 3,
        },
    }
}

fn biased_wake_mutation(value: u8) -> WakeMutationSpec {
    if value & 1 == 0 {
        WakeMutationSpec::None
    } else {
        WakeMutationSpec::PrependStaticRoot { tag: value }
    }
}

fn biased_fragment_key_mode(value: u8) -> FragmentKeyMode {
    if value & 1 == 0 {
        FragmentKeyMode::Unkeyed
    } else {
        FragmentKeyMode::Keyed { base: value }
    }
}

fn biased_attr(value: u8) -> AttrSpec {
    let attr_value = match value % 7 {
        0 => AttrValueSpec::Text(value),
        1 => AttrValueSpec::Float(value),
        2 => AttrValueSpec::Int(value),
        3 => AttrValueSpec::Bool(value % 2 == 0),
        4 => AttrValueSpec::Any(value),
        5 => AttrValueSpec::None,
        _ => AttrValueSpec::Listener,
    };
    AttrSpec {
        name: biased_dynamic_attr_name(&attr_value, value),
        namespace: None,
        value: attr_value,
        volatile: false,
    }
}

fn biased_dynamic_attr_name(value: &AttrValueSpec, seed: u8) -> u8 {
    // Listeners use a name format that's keyed by slot, not by this byte's
    // value — leave the existing `seed & 0x7f` selection alone.
    if matches!(value, AttrValueSpec::Listener) {
        return seed & 0x7f;
    }

    let raw = match value {
        AttrValueSpec::Text(value)
        | AttrValueSpec::Float(value)
        | AttrValueSpec::Int(value)
        | AttrValueSpec::Any(value) => *value,
        AttrValueSpec::Bool(value) => u8::from(*value),
        AttrValueSpec::None => 0,
        AttrValueSpec::Listener => unreachable!("handled by the early return above"),
    };

    // Allow a small fraction of out-of-pool names through so the
    // "no static at this key" diff path keeps getting exercised.
    if seed & 0xF0 == 0xF0 {
        seed
    } else {
        raw & ATTR_NAME_POOL_MASK
    }
}

fn shrink_case(candidates: &mut Candidates<'_>, case: &mut FuzzCase) -> MutatisResult<()> {
    let len = case.ops.len();

    if len > 1 {
        candidates.mutation(|context| {
            random_multistep_shrink_case(case, context.rng());
            Ok(())
        })?;

        candidates.mutation_group((len - 1) as u32, |_context, which| {
            case.ops.truncate(which as usize + 1);
            Ok(())
        })?;

        let chunk_sizes = chunk_delete_sizes(len);
        let delete_count = chunk_sizes
            .iter()
            .map(|size| len.saturating_sub(*size) + 1)
            .sum::<usize>();
        candidates.mutation_group(delete_count as u32, |_context, mut which| {
            for size in chunk_sizes {
                let starts = len - size + 1;
                if which < starts as u32 {
                    let start = which as usize;
                    case.ops.drain(start..start + size);
                    return Ok(());
                }
                which -= starts as u32;
            }
            Ok(())
        })?;
    }

    for index in 0..len {
        let replacements = simplified_ops(&case.ops[index]);
        if replacements.is_empty() {
            continue;
        }

        candidates.mutation_group(replacements.len() as u32, |_context, which| {
            case.ops[index] = replacements[which as usize].clone();
            Ok(())
        })?;
    }

    let mut op_mutator = mutatis::mutators::default::<Op>();
    for op in &mut case.ops {
        op_mutator.mutate(candidates, op)?;
    }

    Ok(())
}

fn chunk_delete_sizes(len: usize) -> Vec<usize> {
    let mut sizes = Vec::new();
    let mut size = len / 2;
    while size > 1 {
        if !sizes.contains(&size) {
            sizes.push(size);
        }
        size /= 2;
    }
    sizes.push(1);
    sizes
}

#[derive(Debug)]
pub struct FuzzFailure {
    step: usize,
    op: String,
    message: String,
}

impl fmt::Display for FuzzFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let summary = self.message.lines().next().unwrap_or(&self.message);
        write!(
            f,
            "fuzz case failed at step {} while applying {}: {}",
            self.step, self.op, summary
        )
    }
}

pub fn format_failure_report(case: &FuzzCase, failure: &FuzzFailure) -> String {
    let mut report = String::new();
    let summary = failure.message.lines().next().unwrap_or(&failure.message);

    use fmt::Write;
    writeln!(&mut report, "fuzz failure").unwrap();
    writeln!(&mut report, "decoded operations: {}", case.ops.len()).unwrap();
    writeln!(&mut report, "failed at step: {}", failure.step).unwrap();
    writeln!(&mut report, "failing op: {}", failure.op).unwrap();
    writeln!(&mut report, "summary: {summary}").unwrap();
    writeln!(&mut report).unwrap();
    writeln!(&mut report, "operations:").unwrap();
    for (index, op) in case.ops.iter().enumerate() {
        let marker = if index == failure.step { ">>" } else { "  " };
        writeln!(&mut report, "{marker} {index:03}: {op:?}").unwrap();
    }
    writeln!(&mut report).unwrap();
    writeln!(&mut report, "full error:").unwrap();
    for line in failure.message.lines() {
        writeln!(&mut report, "  {line}").unwrap();
    }

    report
}

#[derive(Serialize)]
struct EncodedFuzzCase<'a> {
    ops: &'a [Op],
}

#[derive(Deserialize)]
struct DecodedFuzzCase {
    ops: Vec<Op>,
}

pub fn decode_case(data: &[u8]) -> Option<FuzzCase> {
    let decoded = postcard::from_bytes::<DecodedFuzzCase>(data).ok()?;
    let mut case = FuzzCase::new(decoded.ops);
    case.normalize();
    Some(case)
}

pub fn encode_case(case: &FuzzCase, data: &mut [u8], max_size: usize) -> Option<usize> {
    let size = max_size.min(data.len());
    let encoded =
        postcard::to_slice(&EncodedFuzzCase { ops: &case.ops }, &mut data[..size]).ok()?;
    Some(encoded.len())
}

fn encode_case_vec(case: &FuzzCase) -> Option<Vec<u8>> {
    postcard::to_allocvec(&EncodedFuzzCase { ops: &case.ops }).ok()
}

pub fn reduce_case_to_encoded_vec(
    case: &FuzzCase,
    encoded_len: usize,
    max_size: usize,
    options: ReductionOptions,
) -> Option<Vec<u8>> {
    reducer::reduce_case_to_encoded_vec(case, encoded_len, max_size, options)
}

/// Drive the VirtualDom's pending work to completion synchronously.
fn drive_render(dom: &mut dioxus_core::VirtualDom) {
    dom.render_immediate(&mut dioxus_core::Mutations::default());
}

thread_local! {
    /// Shared generation counter for the `warmup_*` scenarios below. Apps read
    /// it via [`warmup_gen`] to pick which variant to render;
    /// [`run_generations`] advances it once per render round.
    static WARMUP_GEN: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// The current warmup generation: 0 during the initial rebuild, then 1, 2, …
/// for each subsequent render round driven by [`run_generations`].
fn warmup_gen() -> u32 {
    WARMUP_GEN.with(|c| c.get())
}

/// Run a warmup app through `generations` render rounds: reset [`WARMUP_GEN`]
/// to 0, rebuild against a fresh [`RendererOracle`], then for each generation
/// `g` in `1..generations` set `WARMUP_GEN = g`, mark the root scope dirty,
/// and render. Returns the dom and oracle so callers can drive extra custom
/// rounds.
fn run_generations(
    app: fn() -> dioxus_core::Element,
    generations: u32,
) -> (
    dioxus_core::VirtualDom,
    dioxus_renderer_oracle::RendererOracle,
) {
    use dioxus_core::{ScopeId, VirtualDom};
    use dioxus_renderer_oracle::RendererOracle;

    WARMUP_GEN.with(|c| c.set(0));
    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    for g in 1..generations {
        WARMUP_GEN.with(|c| c.set(g));
        dom.mark_dirty(ScopeId::APP);
        oracle.render(&mut dom);
    }
    (dom, oracle)
}

/// Drive a small unkeyed fragment of identical-component children through a
/// re-render so the batched `queue_component_props_diff` fast path in
/// `diff::iterator::diff_child_pairs` fires (every pair is a same-component,
/// same-render-fn match, exceeding `FRAGMENT_WORK_BATCH`). Also exercises
/// the `Take` iterator monomorphization via a keyed shared-prefix re-render.
fn warmup_batched_component_props_diff() {
    use dioxus::prelude::*;

    #[derive(Clone, PartialEq, Props)]
    struct ItemProps {
        value: u32,
    }

    #[allow(non_snake_case)]
    fn Item(props: ItemProps) -> Element {
        rsx! { span { "{props.value}" } }
    }

    // --- Unkeyed: exercises the slice-iter monomorphization of
    // `diff_child_pairs`.
    fn unkeyed_app() -> Element {
        let g = warmup_gen();
        rsx! {
            for i in 0..20u32 {
                Item { value: i + g }
            }
        }
    }
    run_generations(unkeyed_app, 2);

    // --- Keyed with stable prefix: exercises the `Take<slice iter>`
    // monomorphization of `diff_child_pairs` reached via
    // `diff_shared_prefix` in `diff_keyed_children`. Keep the first
    // `FRAGMENT_WORK_BATCH + 1` keys stable so the shared-prefix walk pumps
    // a same-component batched diff through the fast path.
    fn keyed_app() -> Element {
        let g = warmup_gen();
        rsx! {
            for i in 0..20u32 {
                Item { key: "{i}", value: i + g }
            }
        }
    }
    run_generations(keyed_app, 2);
}

/// Drive a keyed shuffle of >FRAGMENT_WORK_BATCH items so
/// `diff_keyed_middle`'s `collect_splice_mounts` walks survivors that exercise
/// the `if old_mount.mounted()` branch on the slice picked by the LIS-based
/// splice.
fn warmup_keyed_reorder() {
    use dioxus::prelude::*;

    #[derive(Clone, PartialEq, Props)]
    struct ItemProps {
        value: u32,
    }

    #[allow(non_snake_case)]
    fn Item(props: ItemProps) -> Element {
        rsx! { span { "{props.value}" } }
    }

    fn keyed_shuffle_app() -> Element {
        let round = warmup_gen();
        // Build a permutation of 0..20 that's the identity on round 0 and
        // shuffled on round 1+. The shuffled half forces `diff_keyed_middle`
        // to splice survivors, walking through `collect_splice_mounts`.
        let order: Vec<u32> = if round == 0 {
            (0..20u32).collect()
        } else {
            (0..20u32).rev().collect()
        };
        rsx! {
            for key in order.iter().copied() {
                Item { key: "{key}", value: key }
            }
        }
    }
    run_generations(keyed_shuffle_app, 2);
}

/// Drive a `SuspenseBoundary` through suspend/resolve transitions so the
/// "hidden subtree" defensive paths fire: vnodes whose mount is
/// `PLACEHOLDER` because they live in the suspended branch and were never
/// materialized in the renderer arena.
fn warmup_suspense_hidden_paths() {
    use dioxus::prelude::*;
    use dioxus_core::generation;
    use std::cell::Cell;

    thread_local! {
        static SUSPEND_GEN: Cell<usize> = const { Cell::new(usize::MAX) };
        static SHUFFLE_GEN: Cell<usize> = const { Cell::new(usize::MAX) };
    }

    #[derive(Clone, PartialEq, Props)]
    struct ChildProps {
        value: u32,
    }

    #[component]
    #[allow(non_snake_case)]
    fn SuspendingChild(props: ChildProps) -> Element {
        let g = generation();
        let suspend_at = SUSPEND_GEN.with(|c| c.get());
        if g == suspend_at {
            let task = spawn(async { std::future::pending::<()>().await });
            suspend(task)?;
        }
        rsx! { span { "{props.value}" } }
    }

    // Scenario A: suspend on first render, then re-render so the boundary
    // re-diffs its background children whose mounts may be `PLACEHOLDER`.
    {
        SUSPEND_GEN.with(|c| c.set(0));
        fn app_a() -> Element {
            rsx! {
                SuspenseBoundary {
                    fallback: |_| rsx! { "loading" },
                    for i in 0..20u32 {
                        SuspendingChild { key: "{i}", value: i }
                    }
                }
            }
        }
        run_generations(app_a, 4);
    }

    // Scenario B: render normally, then suspend, then re-render with a
    // reversed key order. The keyed-reorder path observes children in the
    // suspended branch with non-mounted state, exercising the defensive
    // `mounted()` checks in `collect_splice_mounts` and the
    // `mount.as_usize()?` early return in `component_props_update`.
    {
        SUSPEND_GEN.with(|c| c.set(1));
        SHUFFLE_GEN.with(|c| c.set(2));
        fn app_b() -> Element {
            let shuffle_at = SHUFFLE_GEN.with(|c| c.get());
            let g = generation();
            let keys: Vec<u32> = if g >= shuffle_at {
                (0..20u32).rev().collect()
            } else {
                (0..20u32).collect()
            };
            rsx! {
                SuspenseBoundary {
                    fallback: |_| rsx! { "loading" },
                    for key in keys.iter().copied() {
                        SuspendingChild { key: "{key}", value: key }
                    }
                }
            }
        }
        // generation 1: suspend; generation 2: shuffle + still suspending;
        // generation 3: shuffle again.
        run_generations(app_b, 4);
    }
    // Reset for any subsequent warmups.
    SUSPEND_GEN.with(|c| c.set(usize::MAX));
    SHUFFLE_GEN.with(|c| c.set(usize::MAX));
}

/// Mark a parent scope and all of its descendant scopes dirty at once, then
/// drive a render. Exercises the scheduler diffing an ancestor whose children
/// are also queued, so the descendants are drained as part of the ancestor's
/// pass instead of being re-run afterwards.
fn warmup_deferred_subtree_check() {
    use dioxus::prelude::*;
    use dioxus_core::{ScopeId, VirtualDom};

    #[derive(Clone, PartialEq, Props)]
    struct ChildProps {
        value: u32,
    }

    #[allow(non_snake_case)]
    fn Child(props: ChildProps) -> Element {
        rsx! { span { "{props.value}" } }
    }

    fn app() -> Element {
        rsx! {
            for i in 0..5u32 {
                Child { value: i }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    // Cover all plausible scope ids for the Child instances. Unmounted ids
    // are silently ignored by `mark_dirty`.
    for scope_idx in 1usize..=10 {
        dom.mark_dirty(ScopeId(scope_idx));
    }
    drive_render(&mut dom);
}

/// Mix of nested suspense and partial removal: builds a stack of components
/// inside a suspense boundary, then removes/re-orders entries while some
/// remain suspended. The intent is to leave the diff machinery holding a
/// stale `ScopeId` from a dropped sibling so the `get_scope(_)?` and
/// `try_root_node()?` early-returns in
/// `dynamic_node_first_element`/`find_element_at_root_in_target` actually
/// take their `None` branches.
fn warmup_dropped_scope_anchor_lookup() {
    use dioxus::prelude::*;
    use dioxus_core::generation;

    #[derive(Clone, PartialEq, Props)]
    struct InnerProps {
        value: u32,
    }

    #[component]
    #[allow(non_snake_case)]
    fn Suspender(props: InnerProps) -> Element {
        if warmup_gen() == 1 {
            let task = spawn(async { std::future::pending::<()>().await });
            suspend(task)?;
        }
        rsx! { span { "{props.value}" } }
    }

    fn app() -> Element {
        let g = generation();
        let n = match g {
            0 => 10u32,
            1 => 10,
            2 => 4,
            3 => 0,
            _ => 0,
        };
        if n == 0 {
            return rsx! { "empty" };
        }
        rsx! {
            SuspenseBoundary {
                fallback: |_| rsx! { "loading" },
                for i in 0..n {
                    Suspender { key: "{i}", value: i }
                }
            }
        }
    }

    // gen 1: suspend; gen 2: shrink the suspended fragment from 10 to 4
    // (drops 6 suspended child scopes); gen 3: remove the boundary entirely.
    run_generations(app, 4);
}

/// Suspense + removal: render a suspense boundary, suspend its child, then
/// fully remove the boundary so the hidden subtree's vnodes get removed via
/// `remove_node_inner` with `PLACEHOLDER` mounts. Exercises the
/// `!mount.mounted()` early-return in `remove_node_inner` plus the `?`
/// operators in `dynamic_node_first_element`/`find_element_at_root_in_target`
/// when scopes get dropped mid-diff.
fn warmup_suspense_then_remove() {
    use dioxus::prelude::*;

    #[derive(Clone, PartialEq, Props)]
    struct ChildProps {
        value: u32,
    }

    #[component]
    #[allow(non_snake_case)]
    fn SuspendForever(props: ChildProps) -> Element {
        let task = spawn(async { std::future::pending::<()>().await });
        suspend(task)?;
        rsx! { span { "{props.value}" } }
    }

    fn app() -> Element {
        if warmup_gen() >= 2 {
            // After the remove gen, render nothing — the boundary and its
            // suspended subtree get fully removed.
            return rsx! { "removed" };
        }
        rsx! {
            SuspenseBoundary {
                fallback: |_| rsx! { "loading" },
                for i in 0..10u32 {
                    SuspendForever { key: "{i}", value: i }
                }
            }
        }
    }

    // generation 1: re-render, boundary stays suspended; generation 2:
    // replace the boundary with plain text — removes the suspended subtree,
    // exercising remove_node_inner on `PLACEHOLDER` mounts in the hidden
    // children.
    run_generations(app, 3);
}

/// One-shot warmup that exercises the multi-priority deferred-priority paths in
/// `dioxus_core::diff::component::diff_vcomponent`. The sync `render_immediate`
/// path used by [`run_case`] only ever processes a single priority level at a
/// time, so the `render_deferred_priority`/`deferred_priority_for_subtree`
/// branches are unreachable from corpus inputs alone. Calling this once per
/// fuzz process records coverage of those branches in the fuzz binary.
/// Drive a `Portal` through a target-switch transition so the retarget
/// branch of the portal driver's diff and the surrounding
/// `remove_node_inner` + `create_children_with_parents` machinery fire. The
/// fuzz harness's per-input Portal always uses a single target allocated via
/// `use_hook`, so this branch is otherwise unreachable.
fn warmup_portal_target_switch() {
    use dioxus::prelude::*;
    use dioxus_core::{Portal, RenderTargetId, ScopeId, VirtualDom};
    use dioxus_renderer_oracle::{MultiTargetWriter, RendererOracle};
    use std::cell::Cell;

    thread_local! {
        static MODE: Cell<u32> = const { Cell::new(0) };
        static FIRST_TARGET: Cell<u64> = const { Cell::new(0) };
        static SECOND_TARGET: Cell<u64> = const { Cell::new(0) };
    }

    fn app() -> Element {
        let mode = MODE.with(|c| c.get());
        let target = match mode {
            0 | 2 => RenderTargetId(FIRST_TARGET.with(|c| c.get()) as usize),
            _ => RenderTargetId(SECOND_TARGET.with(|c| c.get()) as usize),
        };
        rsx! {
            Portal {
                target,
                span { "portal body" }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let first = dom.runtime().create_render_target();
    let second = dom.runtime().create_render_target();
    FIRST_TARGET.with(|c| c.set(first.0 as u64));
    SECOND_TARGET.with(|c| c.set(second.0 as u64));
    let mut writer = MultiTargetWriter::<RendererOracle>::new();
    writer.insert(RenderTargetId::ROOT, RendererOracle::new());
    writer.insert(first, RendererOracle::new());
    writer.insert(second, RendererOracle::new());
    dom.rebuild(&mut writer);

    // mode 1: switch from first -> second target, with oracles attached.
    MODE.with(|c| c.set(1));
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut writer);

    // mode 2: switch back to first target with NO oracle attached for it.
    // `render_target_should_write` still returns true (target is Real), but
    // the writer reports it unready — this drives the `render_to` filter
    // chain and the `if let Some(to) = render_to` alternative branches.
    let _ = writer.take(first);
    let _ = writer.take(second);
    MODE.with(|c| c.set(2));
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut writer);

    // mode 3: same props as mode 2 — memoize sees self == new and the
    // `equal` branch of `PortalProps::memoize` fires.
    MODE.with(|c| c.set(2));
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut writer);

    // Separate dom: switch to a NOOP target so `render_target_should_write`
    // returns false and the `should_mount` / `if let Some(to)` false arms
    // of the target-switch branch fire.
    drop(dom);
    let mut dom = VirtualDom::new(app);
    let first = dom.runtime().create_render_target();
    let noop = dom.runtime().create_noop_render_target();
    FIRST_TARGET.with(|c| c.set(first.0 as u64));
    SECOND_TARGET.with(|c| c.set(noop.0 as u64));
    MODE.with(|c| c.set(0));
    let mut writer = MultiTargetWriter::<RendererOracle>::new();
    writer.insert(RenderTargetId::ROOT, RendererOracle::new());
    writer.insert(first, RendererOracle::new());
    dom.rebuild(&mut writer);
    MODE.with(|c| c.set(1));
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut writer);
}

/// Mount a scope with a pending effect, then drop it. Exercises the
/// `drop_scope` filter closure that drains `pending_effects` entries for
/// the dropped subtree — unreachable from the fuzz harness because the
/// model never uses `use_effect`.
fn warmup_scope_with_pending_effect() {
    use dioxus::prelude::*;
    use dioxus_core::{ScopeId, current_scope_id, queue_effect};
    use std::cell::Cell;

    thread_local! {
        static CHILD_SCOPE: Cell<Option<ScopeId>> = const { Cell::new(None) };
        static GRANDCHILD_SCOPE: Cell<Option<ScopeId>> = const { Cell::new(None) };
    }

    #[component]
    #[allow(non_snake_case)]
    fn Grandchild() -> Element {
        use_hook(|| {
            GRANDCHILD_SCOPE.with(|c| c.set(Some(current_scope_id())));
        });
        rsx! { em { "grandchild" } }
    }

    #[component]
    #[allow(non_snake_case)]
    fn EffectChild() -> Element {
        use_hook(|| {
            CHILD_SCOPE.with(|c| c.set(Some(current_scope_id())));
        });
        rsx! { span { Grandchild {} } }
    }

    fn app() -> Element {
        if warmup_gen() == 0 {
            rsx! { EffectChild {} }
        } else {
            rsx! { "no child" }
        }
    }

    let (mut dom, mut renderer) = run_generations(app, 1);

    let child_id = CHILD_SCOPE.with(|c| c.get()).expect("child scope captured");
    let grandchild_id = GRANDCHILD_SCOPE
        .with(|c| c.get())
        .expect("grandchild scope captured");

    // Inject pending effects for both the child and the grandchild so the
    // descendant arm of the `drop_scope` filter (id == effect.order.id) and
    // the `is_descendant_of` arm both fire when the parent is unmounted.
    let runtime = dom.runtime();
    runtime.in_scope(child_id, || {
        queue_effect(|| {});
    });
    runtime.in_scope(grandchild_id, || {
        queue_effect(|| {});
    });

    // Removing the child triggers `drop_scope(child)`, which then sees its own
    // and its descendant's pending effects and removes their stale entries.
    WARMUP_GEN.with(|c| c.set(1));
    dom.mark_dirty(ScopeId::APP);
    renderer.render(&mut dom);
}

/// Drive `use_before_render` and `use_after_render` hooks so the pre/post-render
/// closure loops in `run_scope` actually iterate something. The hooks are
/// pushed into the scope's `before_render`/`after_render` lists on the first
/// render, but the loops only see them on subsequent renders — so this warmup
/// captures the child's `ScopeId` on first render and marks the child dirty
/// to force a re-run that actually iterates the hook lists.
fn warmup_before_after_render_hooks() {
    use dioxus::prelude::*;
    use dioxus_core::{ScopeId, current_scope_id, use_after_render, use_before_render};
    use std::cell::Cell;

    thread_local! {
        static HOOKED_SCOPE: Cell<Option<ScopeId>> = const { Cell::new(None) };
    }

    #[component]
    #[allow(non_snake_case)]
    fn HookedChild() -> Element {
        use_before_render(|| {});
        use_after_render(|| {});
        use_hook(|| {
            HOOKED_SCOPE.with(|c| c.set(Some(current_scope_id())));
        });
        rsx! { span { "child" } }
    }

    fn app() -> Element {
        rsx! { HookedChild {} }
    }

    let (mut dom, mut renderer) = run_generations(app, 1);

    if let Some(hooked) = HOOKED_SCOPE.with(|c| c.get()) {
        dom.mark_dirty(hooked);
        renderer.render(&mut dom);
    }
}

/// Drive a component that returns `Err(RenderError::Error(_))` so the error
/// arm in `run_scope`'s `match render_return` and the error arm in
/// `handle_element_return` (which calls `throw_error`) both fire.
fn warmup_throw_error() {
    use dioxus::prelude::*;
    use dioxus_core::{CapturedError, RenderError};

    #[component]
    #[allow(non_snake_case)]
    fn Failing() -> Element {
        Err(RenderError::Error(CapturedError::from_display(
            "expected fuzz error",
        )))
    }

    #[component]
    #[allow(non_snake_case)]
    fn Boundary() -> Element {
        rsx! {
            ErrorBoundary {
                handle_error: |_err: ErrorContext| rsx! { "caught" },
                Failing {}
            }
        }
    }

    run_generations(Boundary, 1);
}

/// Keyed list with a shared left prefix, a fully-replaced middle, and no
/// shared suffix. Drives `diff_keyed_children`'s left-edge splice branch
/// (`right_offset == 0`): the new middle is created anchored against the old
/// left sibling before the old middle is removed.
fn warmup_keyed_left_prefix_splice() {
    use dioxus::prelude::*;

    fn app() -> Element {
        // gen 0: [a, x0, x1, x2]; gen 1: [a, p0, p1, p2].
        // "a" is a shared left prefix, the middle keys are entirely new, and
        // the last keys differ so there is no shared suffix.
        let keys: &[&str] = if warmup_gen() == 0 {
            &["a", "x0", "x1", "x2"]
        } else {
            &["a", "p0", "p1", "p2"]
        };
        rsx! {
            for k in keys.iter().copied() {
                div { key: "{k}", "{k}" }
            }
        }
    }

    run_generations(app, 2);
}

/// A spread adds a dynamic attribute that shadows a static template attribute
/// of the same name, then drops it. Drives the static-template-attribute
/// fallback in `remove_attribute_or_write_fallback`: the removed dynamic
/// attribute's static value is restored instead of cleared.
fn warmup_static_attribute_fallback() {
    use dioxus::prelude::*;
    use dioxus_core::Attribute;

    fn app() -> Element {
        // The template carries a static `class="static"`. The spread shadows
        // it on gen 0, then drops it on gen 1. gen 0 uses a same-namespace
        // (`None`) dynamic attr so the dropped attr restores the static value
        // (the namespace-matches arm of the static lookup); gen 2 uses a
        // *namespaced* dynamic attr so the dropped attr finds the static name
        // but mismatches its namespace (the namespace-mismatch arm).
        let extra: Vec<Attribute> = match warmup_gen() {
            0 => vec![Attribute::new("class", "dynamic", None, false)],
            2 => vec![Attribute::new("class", "dynamic", Some("custom"), false)],
            _ => Vec::new(),
        };
        rsx! {
            div { class: "static", ..extra }
        }
    }

    // gen 1: drop the same-namespace dynamic attr -> restore the static value.
    // gen 2: add a namespaced dynamic attr, gen 3: drop it -> the static
    // lookup finds the name but mismatches the namespace.
    run_generations(app, 4);
}

/// The same spread attribute slot holds a plain value on gen 0 and a listener
/// on gen 1. Drives the `(false, true, Some(_))` arm in
/// `diff_dynamic_attribute`: the old value is explicitly cleared before the
/// listener is installed (installing a listener doesn't overwrite it).
fn warmup_attribute_value_to_listener() {
    use dioxus::prelude::*;
    use dioxus_core::{Attribute, AttributeValue};

    fn app() -> Element {
        let attrs: Vec<Attribute> = if warmup_gen() == 0 {
            vec![Attribute::new("data-x", "value", None, false)]
        } else {
            vec![Attribute::new(
                "data-x",
                AttributeValue::listener(|_: Event<MouseData>| {}),
                None,
                false,
            )]
        };
        rsx! {
            div { ..attrs }
        }
    }

    run_generations(app, 2);
}

/// A keyed list of portals whose body is a single dynamic text node mounted in
/// a *different* render target. Reordering/growing the list makes
/// `push_all_root_nodes` recurse into a portal body whose mount target differs
/// from the list's target, driving the cross-target dynamic-text-root branch.
fn warmup_portal_dynamic_text_root() {
    use dioxus::prelude::*;
    use dioxus_core::{Portal, RenderTargetId, ScopeId, VirtualDom};
    use dioxus_renderer_oracle::{MultiTargetWriter, RendererOracle};
    use std::cell::Cell;

    thread_local! {
        static TARGET: Cell<u64> = const { Cell::new(0) };
    }

    fn app() -> Element {
        let target = RenderTargetId(TARGET.with(|c| c.get()) as usize);
        // Reverse a fully-keyed list (shared keys, no shared prefix/suffix) so
        // `diff_keyed_middle` *moves* most entries through its splice. Each
        // moved entry is re-pushed via `push_all_root_nodes`, which recurses
        // into the portal body — a dynamic text root mounted in `target`.
        let keys: &[u32] = if warmup_gen() == 0 {
            &[0, 1, 2, 3]
        } else {
            &[3, 2, 1, 0]
        };
        rsx! {
            for k in keys.iter().copied() {
                Portal { key: "{k}", target, "{k}" }
            }
        }
    }

    WARMUP_GEN.with(|c| c.set(0));
    let mut dom = VirtualDom::new(app);
    let target = dom.runtime().create_render_target();
    TARGET.with(|c| c.set(target.0 as u64));
    let mut writer = MultiTargetWriter::<RendererOracle>::new();
    writer.insert(RenderTargetId::ROOT, RendererOracle::new());
    writer.insert(target, RendererOracle::new());
    dom.rebuild(&mut writer);

    // gen 1: reverse the list, forcing keyed-middle moves whose
    // `push_all_root_nodes` recurses across the portal's target boundary.
    WARMUP_GEN.with(|c| c.set(1));
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut writer);
}

/// A keyed list of single-element components that is reordered while some
/// entries are removed. During the reorder the diff resolves anchors against
/// component roots whose scopes may already have been dropped, driving the
/// dropped-scope `?` branch in `find_element_at_root_in_target`'s component arm.
fn warmup_keyed_component_anchor() {
    use dioxus::prelude::*;

    #[derive(Clone, PartialEq, Props)]
    struct ItemProps {
        value: u32,
    }

    #[allow(non_snake_case)]
    fn Item(props: ItemProps) -> Element {
        rsx! { span { "{props.value}" } }
    }

    fn app() -> Element {
        // gen 0: [a, b, c, d, e]; gen 1: [a, e] — the middle three component
        // scopes drop while the list keeps a shared prefix/suffix, so anchors
        // are resolved around components mid-removal.
        let keys: &[u32] = if warmup_gen() == 0 {
            &[0, 1, 2, 3, 4]
        } else {
            &[0, 4]
        };
        rsx! {
            for k in keys.iter().copied() {
                Item { key: "{k}", value: k }
            }
        }
    }

    run_generations(app, 2);
}

pub fn warmup_deferred_priority_paths() {
    warmup_batched_component_props_diff();
    warmup_keyed_reorder();
    warmup_suspense_hidden_paths();
    warmup_suspense_then_remove();
    warmup_dropped_scope_anchor_lookup();
    warmup_portal_target_switch();
    warmup_scope_with_pending_effect();
    warmup_before_after_render_hooks();
    warmup_throw_error();
    warmup_deferred_subtree_check();
    warmup_keyed_left_prefix_splice();
    warmup_static_attribute_fallback();
    warmup_attribute_value_to_listener();
    warmup_portal_dynamic_text_root();
    warmup_keyed_component_anchor();
    use dioxus::prelude::*;
    use dioxus_core::ScopeId;

    #[derive(Clone, PartialEq, Props)]
    struct ItemProps {
        value: u32,
    }

    #[allow(non_snake_case)]
    fn Item(props: ItemProps) -> Element {
        rsx! { span { "{props.value}" } }
    }

    fn app() -> Element {
        let generation = dioxus_core::generation();
        rsx! {
            for i in 0..3u32 {
                Item { value: i + (generation as u32) }
            }
        }
    }

    // Re-render a parent whose children are also dirty, driving the diff for a
    // small fragment of identical components.
    {
        let (mut dom, _oracle) = run_generations(app, 1);
        dom.mark_dirty(ScopeId::APP);
        drive_render(&mut dom);
    }
}

pub fn run_case(case: &FuzzCase) -> Result<(), FuzzFailure> {
    let mut state =
        panic::catch_unwind(AssertUnwindSafe(Harness::fresh)).map_err(|payload| FuzzFailure {
            step: 0,
            op: "<initial rebuild>".to_string(),
            message: format!(
                "panic before applying operation: {}",
                panic_message(&payload)
            ),
        })?;

    for (step, op) in case.ops.iter().enumerate() {
        let applied = panic::catch_unwind(AssertUnwindSafe(|| apply_step(&mut state, op)))
            .map_err(|payload| FuzzFailure {
                step,
                op: format!("{op:?}"),
                message: format!(
                    "panic while applying operation: {}",
                    panic_message(&payload)
                ),
            })?;

        applied.map_err(|message| FuzzFailure {
            step,
            op: format!("{op:?}"),
            message,
        })?;
    }
    Ok(())
}

pub fn print_case_trace(case: &FuzzCase, failure: &FuzzFailure) {
    print_ssr_diff_trace(&case.ops, failure.step, &failure.message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_case_roundtrips_and_replays() {
        let case = FuzzCase::default();
        let mut bytes = [0; 4096];
        let size = encode_case(&case, &mut bytes, 4096).unwrap();
        let decoded = decode_case(&bytes[..size]).unwrap();
        assert_eq!(encode_case_vec(&case), encode_case_vec(&decoded));
        run_case(&decoded).unwrap();
    }

    #[test]
    fn biased_primitive_op_replays() {
        for which in 0..PRIMITIVE_MUTATION_COUNT {
            let model = Model::initial();
            let op = biased_primitive_op(&model, which, which as u8, 128 + which as u8);
            run_case(&FuzzCase::new(vec![op])).unwrap();
        }
    }

    #[test]
    fn primitive_dynamic_ops_from_initial_model_are_meaningful() {
        let dynamic_cases = [
            (7, "fragment", 1),
            (8, "leaf", 3),
            (9, "component", 4),
            (13, "suspense_mode", 5),
            (14, "suspense_wake_mutation", 6),
            (15, "wake_suspense", 7),
        ];

        for (which, name, value) in dynamic_cases {
            let mut model = Model::initial();
            let op = biased_primitive_op(&model, which, 0, value);
            ops::apply_strategy_op_to_model(&mut model, &op);
            let dynamic = first_dynamic(&model.root.template.roots)
                .unwrap_or_else(|| panic!("expected dynamic for {name}: {op:?}"));
            assert!(
                !matches!(dynamic, DynamicSpec::Empty),
                "expected non-empty dynamic for {name}: {op:?}"
            );
        }

        let mut model = Model::initial();
        let op = biased_primitive_op(&model, 12, 0, 9);
        ops::apply_strategy_op_to_model(&mut model, &op);
        let attrs = first_dynamic_attrs(&model.root.template.roots)
            .unwrap_or_else(|| panic!("expected dynamic attrs: {op:?}"));
        assert!(
            !attrs.is_empty(),
            "expected non-empty dynamic attrs: {op:?}"
        );
    }

    fn first_dynamic(nodes: &[TemplateNodeSpec]) -> Option<&DynamicSpec> {
        for node in nodes {
            match node {
                TemplateNodeSpec::Element { children, .. } => {
                    if let Some(dynamic) = first_dynamic(children) {
                        return Some(dynamic);
                    }
                }
                TemplateNodeSpec::Text(_) => {}
                TemplateNodeSpec::Dynamic(dynamic) => return Some(dynamic),
            }
        }
        None
    }

    fn first_dynamic_attrs(nodes: &[TemplateNodeSpec]) -> Option<&[AttrSpec]> {
        for node in nodes {
            let TemplateNodeSpec::Element {
                attrs, children, ..
            } = node
            else {
                continue;
            };

            for attr in attrs {
                if let TemplateAttrSpec::Dynamic(attrs) = attr {
                    return Some(attrs);
                }
            }

            if let Some(attrs) = first_dynamic_attrs(children) {
                return Some(attrs);
            }
        }
        None
    }

    #[test]
    fn biased_primitive_op_replays_after_prefix() {
        let prefix = vec![
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                        children: 1,
                        key_base: Some(7),
                    }),
                },
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: Some(7),
                }),
            ),
            Op::template(
                1,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
                    },
                },
            ),
            Op::dynamic(1, 0, suspense_kind(SuspenseMode::Ready { wake_after: 0 })),
        ];
        let model = replay_model_prefix(&prefix, prefix.len());
        for which in 0..PRIMITIVE_MUTATION_COUNT {
            let mut ops = prefix.clone();
            ops.push(biased_primitive_op(
                &model,
                which,
                64 + which as u8,
                192 + which as u8,
            ));
            run_case(&FuzzCase::new(ops)).unwrap();
        }
    }

    #[test]
    fn targeted_diff_coverage_cases_replay() {
        for (name, case) in targeted_diff_coverage_cases() {
            run_case(&case).unwrap_or_else(|failure| {
                panic!("targeted diff coverage case {name:?} failed: {failure}")
            });
        }
    }

    /// Write every targeted diff-coverage case as a postcard-encoded seed file
    /// in the libfuzzer corpus directory. The fuzz binary picks these up on
    /// the next run so the deterministic high-value patterns we hand-built
    /// here also contribute to the coverage-measured fuzz binary, not just
    /// the in-process replay test.
    ///
    /// Gated on `DIOXUS_FUZZ_WRITE_SEEDS=1` so a normal `cargo test` doesn't
    /// mutate corpus state. Run with:
    ///   `DIOXUS_FUZZ_WRITE_SEEDS=1 cargo test -p dioxus-vdom-fuzz --lib write_targeted_seeds_to_corpus -- --nocapture`
    #[test]
    fn write_targeted_seeds_to_corpus() {
        if std::env::var_os("DIOXUS_FUZZ_WRITE_SEEDS").is_none() {
            return;
        }
        let corpus_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fuzz")
            .join("corpus")
            .join("vdom_ops");
        std::fs::create_dir_all(&corpus_dir).unwrap();
        for (name, case) in targeted_diff_coverage_cases() {
            let mut buf = vec![0u8; 64 * 1024];
            let cap = buf.len();
            let len = encode_case(&case, &mut buf, cap)
                .unwrap_or_else(|| panic!("failed to encode {name}"));
            let path = corpus_dir.join(format!("targeted-{name}"));
            std::fs::write(&path, &buf[..len])
                .unwrap_or_else(|err| panic!("failed to write seed {name}: {err}"));
            println!("wrote {}", path.display());
        }
    }

    fn targeted_diff_coverage_cases() -> Vec<(&'static str, FuzzCase)> {
        vec![
            case(
                "non_keyed_append_remove_equal",
                non_keyed_append_remove_equal(),
            ),
            case("keyed_append", keyed_append()),
            case("keyed_prepend", keyed_prepend()),
            case("keyed_remove_and_add_middle", keyed_remove_and_add_middle()),
            case("keyed_replace_all_keys", keyed_replace_all_keys()),
            case("keyed_reorder_insert_remove", keyed_reorder_insert_remove()),
            case("move_static_root", move_root_node_with_kind(None)),
            case(
                "move_dynamic_text_root",
                move_root_node_with_kind(Some(DynamicKind::Text(7))),
            ),
            case(
                "move_dynamic_placeholder_root",
                move_root_node_with_kind(Some(DynamicKind::Placeholder)),
            ),
            case(
                "move_dynamic_fragment_root",
                move_root_node_with_kind(Some(DynamicKind::Fragment {
                    children: 1,
                    key_base: None,
                })),
            ),
            case(
                "move_dynamic_component_root",
                move_root_node_with_kind(Some(DynamicKind::ComponentA)),
            ),
            case("replace_component_render_fn", replace_component_render_fn()),
            case(
                "hidden_suspense_component_removal",
                hidden_suspense_component_removal(),
            ),
            case("suspense_clear_and_reclaim", suspense_clear_and_reclaim()),
            case(
                "dynamic_attribute_transitions",
                dynamic_attribute_transitions(),
            ),
            case("hidden_suspense_text_diff", {
                vec![
                    Op::dynamic(0, 0, suspense_kind(SuspenseMode::Resolved)),
                    set_vnode_root_dynamic(1, DynamicKind::Text(0)),
                    Op::Rerender,
                    Op::suspense(0, SuspenseMode::Ready { wake_after: 0 }),
                    Op::Rerender,
                    set_vnode_root_dynamic(1, DynamicKind::Text(1)),
                    Op::Rerender,
                    Op::wake_suspense(0),
                ]
            }),
            case("hidden_suspense_keyed_fragment_diff", {
                vec![
                    Op::dynamic(0, 0, suspense_kind(SuspenseMode::Resolved)),
                    set_vnode_root_dynamic(
                        1,
                        DynamicKind::Fragment {
                            children: 3,
                            key_base: Some(33),
                        },
                    ),
                    Op::Rerender,
                    Op::suspense(0, SuspenseMode::Ready { wake_after: 0 }),
                    Op::Rerender,
                    insert_fragment_child(2, Some(36)),
                    Op::Rerender,
                    Op::wake_suspense(0),
                ]
            }),
            case(
                "dynamic_attribute_static_fallback",
                dynamic_attribute_static_fallback_recipe(),
            ),
            case("portal_create_and_remove", {
                vec![
                    set_vnode_root_dynamic(0, DynamicKind::Portal),
                    Op::Rerender,
                    set_vnode_root_dynamic(0, DynamicKind::Empty),
                    Op::Rerender,
                ]
            }),
            case("portal_with_text_child_then_rerender", {
                vec![
                    set_vnode_root_dynamic(0, DynamicKind::Portal),
                    Op::Rerender,
                    set_vnode_root_dynamic(0, DynamicKind::Text(7)),
                    Op::Rerender,
                ]
            }),
            case("keyed_portal_text_reorder", {
                // A keyed fragment whose first child is a Portal with a
                // dynamic-text body, reordered so the moved entry is
                // re-pushed via `push_all_root_nodes`. The portal body's
                // text root is mounted in another render target, driving
                // the cross-target `else` arm of the Text case.
                vec![
                    set_root_dynamic(),
                    fragment_with_children(3, Some(40)),
                    set_vnode_root_dynamic(1, DynamicKind::Portal),
                    set_vnode_root_dynamic(2, DynamicKind::Text(7)),
                    set_vnode_root_dynamic(3, DynamicKind::Portal),
                    set_vnode_root_dynamic(4, DynamicKind::Text(8)),
                    set_vnode_root_dynamic(5, DynamicKind::Portal),
                    set_vnode_root_dynamic(6, DynamicKind::Text(9)),
                    Op::Rerender,
                    move_fragment_child(0, 2),
                    move_fragment_child(0, 1),
                    Op::Rerender,
                ]
            }),
            case("portal_inside_fragment_and_remove", {
                vec![
                    set_root_dynamic(),
                    insert_fragment_child(0, None),
                    Op::Rerender,
                    set_vnode_root_dynamic(1, DynamicKind::Portal),
                    Op::Rerender,
                    remove_fragment_child(0),
                    Op::Rerender,
                ]
            }),
            case("unkeyed_fragment_batched_component_props_diff", {
                // Build an unkeyed fragment whose children are all
                // ComponentA. With >FRAGMENT_WORK_BATCH (16) same-component
                // children, `diff_child_pairs` exercises the batched
                // `queue_component_props_diff` path in iterator.rs.
                let mut ops = vec![set_root_dynamic()];
                for _ in 0..18 {
                    ops.push(insert_fragment_child(0, None));
                }
                for vnode in 1..=18u8 {
                    ops.push(set_vnode_root_dynamic(vnode, DynamicKind::ComponentA));
                }
                ops.push(Op::Rerender);
                // Mark the root dirty so the unkeyed fragment re-diffs all
                // children (not just creates them).
                ops.push(insert_fragment_child(0, None));
                ops.push(Op::Rerender);
                ops
            }),
        ]
    }

    fn case(name: &'static str, ops: Vec<Op>) -> (&'static str, FuzzCase) {
        (name, FuzzCase::new(ops))
    }

    fn set_root_dynamic() -> Op {
        Op::template(
            0,
            TemplateEdit::SetNode {
                node: 0,
                kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
            },
        )
    }

    fn insert_fragment_child(index: u8, key: Option<u8>) -> Op {
        Op::fragment(
            0,
            0,
            FragmentEdit::Children(ListEdit::Insert { index, item: key }),
        )
    }

    fn remove_fragment_child(index: u8) -> Op {
        Op::fragment(0, 0, FragmentEdit::Children(ListEdit::Remove { index }))
    }

    fn move_fragment_child(from: u8, to: u8) -> Op {
        Op::fragment(0, 0, FragmentEdit::Children(ListEdit::Move { from, to }))
    }

    fn key_fragment(base: u8) -> Op {
        Op::fragment(0, 0, FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base }))
    }

    fn fragment_with_children(count: u8, key_base: Option<u8>) -> Op {
        Op::dynamic(
            0,
            0,
            DynamicKind::Fragment {
                children: count,
                key_base,
            },
        )
    }

    fn set_vnode_root_dynamic(vnode: u8, kind: DynamicKind) -> Op {
        Op::template(
            vnode,
            TemplateEdit::SetNode {
                node: 0,
                kind: TemplateNodeKind::Dynamic(kind),
            },
        )
    }

    fn non_keyed_append_remove_equal() -> Vec<Op> {
        vec![
            set_root_dynamic(),
            insert_fragment_child(0, None),
            insert_fragment_child(1, None),
            Op::Rerender,
            insert_fragment_child(2, None),
            Op::Rerender,
            remove_fragment_child(1),
            Op::Rerender,
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Text(11),
                },
            ),
            Op::Rerender,
        ]
    }

    fn keyed_append() -> Vec<Op> {
        vec![
            set_root_dynamic(),
            fragment_with_children(2, Some(0)),
            Op::Rerender,
            insert_fragment_child(2, Some(2)),
            Op::Rerender,
        ]
    }

    fn keyed_prepend() -> Vec<Op> {
        vec![
            set_root_dynamic(),
            fragment_with_children(2, Some(1)),
            Op::Rerender,
            insert_fragment_child(0, Some(0)),
            Op::Rerender,
        ]
    }

    fn keyed_remove_and_add_middle() -> Vec<Op> {
        vec![
            set_root_dynamic(),
            fragment_with_children(3, Some(0)),
            Op::Rerender,
            remove_fragment_child(1),
            Op::Rerender,
            insert_fragment_child(1, Some(1)),
            Op::Rerender,
        ]
    }

    fn keyed_replace_all_keys() -> Vec<Op> {
        vec![
            set_root_dynamic(),
            fragment_with_children(2, Some(0)),
            Op::Rerender,
            key_fragment(2),
            Op::Rerender,
        ]
    }

    fn keyed_reorder_insert_remove() -> Vec<Op> {
        vec![
            set_root_dynamic(),
            fragment_with_children(5, Some(0)),
            Op::Rerender,
            move_fragment_child(3, 1),
            insert_fragment_child(2, Some(5)),
            remove_fragment_child(4),
            Op::Rerender,
        ]
    }

    fn move_root_node_with_kind(kind: Option<DynamicKind>) -> Vec<Op> {
        let mut ops = vec![set_root_dynamic(), fragment_with_children(4, Some(0))];

        if let Some(kind) = kind {
            ops.push(set_vnode_root_dynamic(3, kind));
            if matches!(ops.last(), Some(Op::Mutate(_))) {
                // The child vnode selected above must materialize its nested fragment
                // before the keyed move so push_all_root_nodes has live roots to collect.
            }
        }

        ops.extend([Op::Rerender, move_fragment_child(2, 0), Op::Rerender]);
        ops
    }

    fn replace_component_render_fn() -> Vec<Op> {
        vec![
            set_root_dynamic(),
            Op::dynamic(0, 0, DynamicKind::ComponentA),
            Op::Rerender,
            Op::dynamic(0, 0, DynamicKind::ComponentB),
            Op::Rerender,
        ]
    }

    fn hidden_suspense_component_removal() -> Vec<Op> {
        vec![
            set_root_dynamic(),
            Op::dynamic(0, 0, suspense_kind(SuspenseMode::Resolved)),
            Op::template(
                1,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                    },
                },
            ),
            Op::template(
                1,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 1,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                    },
                },
            ),
            Op::dynamic(1, 0, suspense_kind(SuspenseMode::Pending)),
            Op::dynamic(1, 1, DynamicKind::ComponentA),
            Op::Rerender,
            Op::template(
                1,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Remove { index: 1 },
                },
            ),
            Op::Rerender,
        ]
    }

    fn suspense_clear_and_reclaim() -> Vec<Op> {
        vec![
            set_root_dynamic(),
            Op::dynamic(0, 0, suspense_kind(SuspenseMode::Ready { wake_after: 0 })),
            set_vnode_root_dynamic(1, DynamicKind::Empty),
            Op::Rerender,
            Op::wake_suspense(0),
            Op::dynamic(1, 0, DynamicKind::ComponentA),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Ready { wake_after: 0 }),
            Op::Rerender,
            Op::wake_suspense(0),
        ]
    }

    fn dynamic_attribute_transitions() -> Vec<Op> {
        vec![
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Static {
                            name: 9,
                            value: 1,
                            namespace: None,
                        },
                    },
                },
            ),
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 1,
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
                    },
                },
            ),
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 9,
                        namespace: None,
                        value: AttrValueSpec::Text(1),
                        volatile: false,
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 9,
                        namespace: None,
                        value: AttrValueSpec::None,
                        volatile: true,
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 1,
                        namespace: None,
                        value: AttrValueSpec::Listener,
                        volatile: false,
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 1,
                        namespace: None,
                        value: AttrValueSpec::Text(2),
                        volatile: false,
                    },
                },
            ),
            Op::Rerender,
        ]
    }
}
