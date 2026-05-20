//! Reusable Dioxus VirtualDom fuzzing harness.
//!
//! The `cargo-fuzz` target feeds encoded [`FuzzCase`] values into this crate.
//! LibFuzzer owns coverage guidance and corpus management; this crate owns the
//! structured operation stream and renderer oracle.

mod cache;
mod harness;
mod model;
mod ops;
mod reducer;
mod vdom;

use harness::{Harness, apply_step, print_ssr_diff_trace};
use model::{
    AttrSpec, AttrValueSpec, DynamicKind, DynamicSpec, FragmentKeyMode, MAX_FRAGMENT_CHILDREN,
    Model, SuspenseMode, TemplateAttrSpec, TemplateNodeKind, TemplateNodeSpec, VNodeSpec,
    WakeMutationSpec,
};
use mutatis::{Candidates, DefaultMutate, Generate, Mutate, Result as MutatisResult};
use ops::{FragmentEdit, ListEdit, Op, TemplateEdit};
pub use reducer::{ReduceError, ReductionOptions, ReductionReport, ReductionStats, reduce_case};
use reducer::{random_multistep_shrink_case, simplified_ops};
use serde::{Deserialize, Serialize};
use std::{cell::Cell, fmt};

pub const MAX_STEPS: usize = 512;
const OPTIMIZED_MUTATION_STRATEGIES: u32 = 26;
const OPTIMIZED_BURST_LIMIT: usize = 6;
const TARGETED_MUTATION_STRATEGIES: [u32; 4] = [11, 14, 16, 23];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FuzzCase {
    ops: Vec<Op>,
}

impl FuzzCase {
    pub(crate) fn new(mut ops: Vec<Op>) -> Self {
        ops.truncate(MAX_STEPS);
        Self { ops }
    }

    pub fn seed() -> Self {
        Self::new(Vec::new())
    }

    pub fn normalize(&mut self) {
        self.ops.truncate(MAX_STEPS);
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Build a copy of this case with the op at `index` removed.
    pub fn without_op(&self, index: usize) -> Self {
        let mut ops = self.ops.clone();
        if index < ops.len() {
            ops.remove(index);
        }
        Self::new(ops)
    }

    /// Build a copy of this case truncated to the first `len` ops.
    pub fn truncated(&self, len: usize) -> Self {
        let mut ops = self.ops.clone();
        ops.truncate(len);
        Self::new(ops)
    }

    /// Build a copy of this case with `start..end` removed.
    pub fn without_range(&self, start: usize, end: usize) -> Self {
        let end = end.min(self.ops.len());
        let start = start.min(end);
        let mut ops = self.ops.clone();
        ops.drain(start..end);
        Self::new(ops)
    }
}

impl Default for FuzzCase {
    fn default() -> Self {
        Self::seed()
    }
}

thread_local! {
    static ACTIVE_RUN_STEP: Cell<Option<usize>> = const { Cell::new(None) };
}

struct ActiveRunStepGuard;

impl ActiveRunStepGuard {
    fn new() -> Self {
        ACTIVE_RUN_STEP.with(|step| step.set(None));
        Self
    }

    fn set(&self, next_step: usize) {
        ACTIVE_RUN_STEP.with(|step| step.set(Some(next_step)));
    }
}

impl Drop for ActiveRunStepGuard {
    fn drop(&mut self) {
        ACTIVE_RUN_STEP.with(|step| step.set(None));
    }
}

pub fn active_run_step() -> Option<usize> {
    ACTIVE_RUN_STEP.with(Cell::get)
}

#[derive(Clone, Debug, Default)]
pub struct FuzzCaseMutator;

impl DefaultMutate for FuzzCase {
    type DefaultMutate = FuzzCaseMutator;
}

impl Mutate<FuzzCase> for FuzzCaseMutator {
    fn mutate(
        &mut self,
        candidates: &mut Candidates<'_>,
        case: &mut FuzzCase,
    ) -> MutatisResult<()> {
        if candidates.shrink() {
            return shrink_case(candidates, case);
        }

        if !candidates.shrink() && case.ops.len() < MAX_STEPS {
            candidates.mutation(|context| {
                let index = context.rng().gen_index(case.ops.len() + 1).unwrap();
                let mut op_mutator = mutatis::mutators::default::<Op>();
                let op = op_mutator.generate(context)?;
                case.ops.insert(index, op);
                Ok(())
            })?;
        }

        if !candidates.shrink() {
            candidates.mutation_group(OPTIMIZED_MUTATION_STRATEGIES, |context, which| {
                insert_optimized_model_aware_ops(context, case, which);
                Ok(())
            })?;
        }

        if !candidates.shrink() {
            candidates.mutation(|context| {
                let which = TARGETED_MUTATION_STRATEGIES[context
                    .rng()
                    .gen_index(TARGETED_MUTATION_STRATEGIES.len())
                    .unwrap_or(0)];
                if !insert_targeted_model_aware_burst(context, case, which) {
                    insert_optimized_model_aware_ops(context, case, which);
                }
                Ok(())
            })?;
        }

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

fn replay_model_prefix(ops: &[Op], len: usize) -> Model {
    let mut model = Model::initial();
    for op in ops.iter().take(len) {
        ops::apply_op_to_model(&mut model, op);
    }
    model
}

fn insert_optimized_model_aware_op(
    context: &mut mutatis::Context,
    case: &mut FuzzCase,
    which: u32,
) {
    let index = context.rng().gen_index(case.ops.len() + 1).unwrap();
    let model = replay_model_prefix(&case.ops, index);
    let selector = context.rng().gen_u8();
    let value = context.rng().gen_u8();
    let op = optimized_model_aware_op(&model, which, selector, value);

    if case.ops.len() < MAX_STEPS {
        case.ops.insert(index, op);
    } else if !case.ops.is_empty() {
        let replace_index = index.min(case.ops.len() - 1);
        case.ops[replace_index] = op;
    }
}

fn insert_optimized_model_aware_ops(
    context: &mut mutatis::Context,
    case: &mut FuzzCase,
    which: u32,
) {
    if insert_targeted_model_aware_burst(context, case, which) {
        return;
    }

    insert_optimized_model_aware_op(context, case, which);

    let burst_len = context.rng().gen_index(OPTIMIZED_BURST_LIMIT).unwrap_or(0);
    for _ in 0..burst_len {
        let which = context
            .rng()
            .gen_index(OPTIMIZED_MUTATION_STRATEGIES as usize)
            .unwrap_or(0) as u32;
        insert_optimized_model_aware_op(context, case, which);
    }
}

fn insert_targeted_model_aware_burst(
    context: &mut mutatis::Context,
    case: &mut FuzzCase,
    which: u32,
) -> bool {
    let index = context.rng().gen_index(case.ops.len() + 1).unwrap();
    let model = replay_model_prefix(&case.ops, index);
    let selector = context.rng().gen_u8();
    let value = context.rng().gen_u8();

    let ops = match which {
        11 => domless_dynamic_placeholder_burst(&model, selector, value),
        14 => keyed_domless_fragment_burst(&model, selector, value, false),
        16 => keyed_domless_fragment_burst(&model, selector, value, true),
        23 => suspense_background_keyed_burst(&model, selector, value),
        _ => None,
    };

    let Some(ops) = ops else {
        return false;
    };
    insert_ops_at(case, index, ops);
    true
}

fn insert_ops_at(case: &mut FuzzCase, index: usize, ops: Vec<Op>) {
    if ops.is_empty() {
        return;
    }

    if case.ops.len() + ops.len() <= MAX_STEPS {
        case.ops.splice(index..index, ops);
        return;
    }

    for (offset, op) in ops.into_iter().enumerate() {
        let replace_index = index.saturating_add(offset);
        if replace_index < case.ops.len() {
            case.ops[replace_index] = op;
        } else if case.ops.len() < MAX_STEPS {
            case.ops.push(op);
        }
    }
}

fn replay_model_with_ops(model: &Model, ops: &[Op]) -> Model {
    let mut model = model.clone();
    for op in ops {
        ops::apply_op_to_model(&mut model, op);
    }
    model
}

fn apply_model_op(model: &mut Model, op: &Op) {
    ops::apply_op_to_model(model, op);
}

fn domless_dynamic_placeholder_burst(model: &Model, selector: u8, value: u8) -> Option<Vec<Op>> {
    if !model.can_grow() {
        return None;
    }

    let facts = ModelFacts::new(model);
    let vnode = facts.select_focus_vnode(selector, value);
    let element = facts.select_element_with_child_capacity(vnode, selector)?;
    let mut ops = Vec::new();
    let mut current = model.clone();

    let insert = Op::template(
        vnode,
        TemplateEdit::Children {
            element,
            edit: ListEdit::Insert {
                index: biased_index(value, facts.child_count(vnode, element)),
                item: TemplateNodeKind::Dynamic,
            },
        },
    );
    apply_model_op(&mut current, &insert);
    ops.push(insert);

    let facts = ModelFacts::new(&current);
    if let Some(slot) = facts.select_nested_domless_slot(selector) {
        ops.push(Op::dynamic(slot.vnode, slot.slot, DynamicKind::Empty));
    }
    ops.push(Op::Rerender);
    Some(ops)
}

fn keyed_domless_fragment_burst(
    model: &Model,
    selector: u8,
    value: u8,
    prefer_existing: bool,
) -> Option<Vec<Op>> {
    let facts = ModelFacts::new(model);
    if prefer_existing {
        if let Some(ops) = move_existing_keyed_domless_fragment(&facts, selector, value, false) {
            return Some(ops);
        }
    }

    let mut ops = Vec::new();
    let mut current = model.clone();
    let facts = ModelFacts::new(&current);
    let vnode = facts.select_focus_vnode(selector, value);
    if !current.can_grow() || !facts.has_dynamic_slots() {
        return move_existing_keyed_domless_fragment(&facts, selector, value, false);
    }

    let slot = facts.select_dynamic_slot(vnode, selector);
    ops.push(Op::dynamic(vnode, slot, DynamicKind::Fragment));
    apply_model_op(&mut current, ops.last().unwrap());

    for child in 0..4 {
        if !current.can_grow() {
            break;
        }
        let facts = ModelFacts::new(&current);
        let fragment = facts.select_fragment(selector);
        ops.push(Op::fragment(
            fragment.vnode,
            fragment.slot,
            FragmentEdit::Children(ListEdit::Insert {
                index: (child as u8).min(fragment.len as u8),
                item: None,
            }),
        ));
        apply_model_op(&mut current, ops.last().unwrap());
    }

    let facts = ModelFacts::new(&current);
    let fragment = facts.select_fragment(selector);
    ops.push(Op::fragment(
        fragment.vnode,
        fragment.slot,
        FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base: value }),
    ));
    apply_model_op(&mut current, ops.last().unwrap());

    let facts = ModelFacts::new(&current);
    let fragment = facts.select_fragment(selector);
    let changed_start = ops.len();
    for child in fragment.select_child_pair(selector) {
        ops.push(Op::template(
            child.vnode,
            TemplateEdit::SetNode {
                node: 0,
                kind: TemplateNodeKind::Dynamic,
            },
        ));
    }
    if ops.len() == changed_start {
        return None;
    }
    ops.push(Op::Rerender);
    current = replay_model_with_ops(&current, &ops[changed_start..]);

    let facts = ModelFacts::new(&current);
    if let Some(mut move_ops) = move_existing_keyed_domless_fragment(&facts, selector, value, true)
    {
        ops.append(&mut move_ops);
    }

    Some(ops)
}

fn move_existing_keyed_domless_fragment(
    facts: &ModelFacts,
    selector: u8,
    value: u8,
    require_domless: bool,
) -> Option<Vec<Op>> {
    let fragment = facts.select_keyed_fragment(selector, require_domless)?;
    if fragment.len < 2 {
        return None;
    }

    let from = fragment
        .select_domless_child(selector)
        .map(|child| child.index)
        .unwrap_or_else(|| biased_existing_index(selector, fragment.len));

    let mut ops = Vec::new();
    for to in adjacent_move_targets(from, fragment.len, value) {
        ops.push(fragment_move_op(fragment, from, to));
        ops.push(Op::Rerender);
    }

    (!ops.is_empty()).then_some(ops)
}

fn adjacent_move_targets(from: u8, len: usize, value: u8) -> Vec<u8> {
    let mut targets = Vec::new();
    let last = len.saturating_sub(1).min(u8::MAX as usize) as u8;
    if from > 0 {
        targets.push(from - 1);
    }
    if from < last {
        targets.push(from + 1);
    }

    let biased = biased_index(value, len);
    if biased != from && !targets.contains(&biased) {
        targets.push(biased);
    }

    targets.truncate(3);
    targets
}

fn fragment_move_op(fragment: FragmentShape, from: u8, to: u8) -> Op {
    Op::fragment(
        fragment.vnode,
        fragment.slot,
        FragmentEdit::Children(ListEdit::Move { from, to }),
    )
}

fn suspense_background_keyed_burst(model: &Model, selector: u8, value: u8) -> Option<Vec<Op>> {
    let facts = ModelFacts::new(model);
    let fragment = facts.select_suspense_keyed_domless_fragment(selector)?;
    if fragment.len < 2 {
        return None;
    }

    let from = fragment
        .select_domless_child(selector)
        .map(|child| child.index)
        .unwrap_or_else(|| biased_existing_index(selector, fragment.len));
    let to = adjacent_move_targets(from, fragment.len, value)
        .into_iter()
        .next()
        .unwrap_or_else(|| biased_index(value, fragment.len));

    Some(vec![
        Op::Rerender,
        Op::suspense(
            fragment
                .suspense
                .unwrap_or_else(|| facts.select_suspense(selector)),
            SuspenseMode::Pending,
        ),
        Op::Rerender,
        fragment_move_op(fragment, from, to),
        Op::Rerender,
    ])
}

fn optimized_model_aware_op(model: &Model, which: u32, selector: u8, value: u8) -> Op {
    let facts = ModelFacts::new(model);
    let vnode = facts.select_focus_vnode(selector, value);
    let node = facts.select_node(vnode, value);
    let element = facts.select_element(vnode, value);
    match which {
        0 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::SetNode {
                node,
                kind: TemplateNodeKind::Dynamic,
            },
        ),
        1 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::SetNode {
                node,
                kind: biased_template_node_kind(value),
            },
        ),
        2 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::Roots {
                edit: ListEdit::Insert {
                    index: biased_index(value, facts.root_count(vnode)),
                    item: biased_template_node_kind(value),
                },
            },
        ),
        3 => Op::template(
            vnode,
            TemplateEdit::Roots {
                edit: remove_or_move_list_edit(facts.root_count(vnode), selector, value),
            },
        ),
        4 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::Children {
                element,
                edit: ListEdit::Insert {
                    index: biased_index(value, facts.child_count(vnode, element)),
                    item: biased_template_node_kind(value),
                },
            },
        ),
        5 => Op::template(
            vnode,
            TemplateEdit::Children {
                element,
                edit: remove_or_move_list_edit(facts.child_count(vnode, element), selector, value),
            },
        ),
        6 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::Attrs {
                element,
                edit: ListEdit::Insert {
                    index: biased_index(value, facts.template_attr_count(vnode, element)),
                    item: biased_template_attr(value),
                },
            },
        ),
        7 => Op::template(
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
        8 if facts.has_dynamic_slots() => Op::dynamic(
            vnode,
            facts.select_dynamic_slot(vnode, selector),
            DynamicKind::Fragment,
        ),
        9 if facts.has_dynamic_slots() => Op::dynamic(
            vnode,
            facts.select_dynamic_slot(vnode, selector),
            biased_leaf_dynamic_kind(value),
        ),
        10 if facts.has_dynamic_slots() => Op::dynamic(
            vnode,
            facts.select_dynamic_slot(vnode, selector),
            if value & 1 == 0 {
                DynamicKind::ComponentA
            } else {
                DynamicKind::ComponentB
            },
        ),
        11 if facts.has_dynamic_slots() => Op::dynamic(
            vnode,
            facts.select_dynamic_slot(vnode, selector),
            DynamicKind::ComponentA,
        ),
        12 if facts.has_dynamic_slots() => Op::dynamic(
            vnode,
            facts.select_dynamic_slot(vnode, selector),
            DynamicKind::Suspense {
                mode: biased_suspense_mode(value),
            },
        ),
        13 if facts.has_dynamic_slots() => {
            let fragment = facts.select_fragment(selector);
            Op::fragment(
                fragment.vnode,
                fragment.slot,
                FragmentEdit::KeyMode(biased_fragment_key_mode(value)),
            )
        }
        14 if model.can_grow() && facts.has_dynamic_slots() => {
            let fragment = facts.select_fragment(selector);
            Op::fragment(
                fragment.vnode,
                fragment.slot,
                FragmentEdit::Children(ListEdit::Insert {
                    index: biased_index(value, fragment.len),
                    item: biased_fragment_child_key(value, fragment.len, fragment.keyed),
                }),
            )
        }
        15 if facts.has_dynamic_slots() => {
            let fragment = facts.select_fragment(selector);
            if fragment.len == 0 && model.can_grow() {
                Op::fragment(
                    fragment.vnode,
                    fragment.slot,
                    FragmentEdit::Children(ListEdit::Insert {
                        index: 0,
                        item: biased_fragment_child_key(value, fragment.len, fragment.keyed),
                    }),
                )
            } else {
                Op::fragment(
                    fragment.vnode,
                    fragment.slot,
                    FragmentEdit::Children(ListEdit::Remove {
                        index: biased_existing_index(value, fragment.len),
                    }),
                )
            }
        }
        16 if facts.has_dynamic_slots() => {
            let fragment = facts.select_fragment(selector);
            if fragment.len < 2 && model.can_grow() {
                Op::fragment(
                    fragment.vnode,
                    fragment.slot,
                    FragmentEdit::Children(ListEdit::Insert {
                        index: biased_index(value, fragment.len),
                        item: biased_fragment_child_key(value, fragment.len, fragment.keyed),
                    }),
                )
            } else {
                Op::fragment(
                    fragment.vnode,
                    fragment.slot,
                    FragmentEdit::Children(ListEdit::Move {
                        from: biased_existing_index(selector, fragment.len),
                        to: biased_index(value, fragment.len),
                    }),
                )
            }
        }
        17 if facts.has_attr_slots() => {
            let attr = facts.select_attr_slot(selector);
            Op::dynamic_attrs(
                attr.vnode,
                attr.slot,
                ListEdit::Insert {
                    index: biased_index(value, attr.len),
                    item: optimized_attr(value),
                },
            )
        }
        17 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::Attrs {
                element,
                edit: ListEdit::Insert {
                    index: biased_index(value, facts.template_attr_count(vnode, element)),
                    item: TemplateAttrSpec::Dynamic,
                },
            },
        ),
        18 if facts.has_attr_slots() => {
            let attr = facts.select_attr_slot(selector);
            Op::dynamic_attrs(
                attr.vnode,
                attr.slot,
                ListEdit::Remove {
                    index: biased_existing_index(value, attr.len),
                },
            )
        }
        18 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::Attrs {
                element,
                edit: ListEdit::Insert {
                    index: biased_index(value, facts.template_attr_count(vnode, element)),
                    item: TemplateAttrSpec::Dynamic,
                },
            },
        ),
        19 if facts.has_attr_slots() => {
            let attr = facts.select_attr_slot(selector);
            Op::dynamic_attrs(
                attr.vnode,
                attr.slot,
                ListEdit::Move {
                    from: biased_existing_index(selector, attr.len),
                    to: biased_index(value, attr.len),
                },
            )
        }
        19 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::Attrs {
                element,
                edit: ListEdit::Insert {
                    index: biased_index(value, facts.template_attr_count(vnode, element)),
                    item: TemplateAttrSpec::Dynamic,
                },
            },
        ),
        20 if facts.has_suspense() => {
            Op::suspense(facts.select_suspense(selector), biased_suspense_mode(value))
        }
        20 if facts.has_dynamic_slots() => Op::dynamic(
            vnode,
            facts.select_dynamic_slot(vnode, selector),
            DynamicKind::Suspense {
                mode: biased_suspense_mode(value),
            },
        ),
        21 if facts.has_suspense() => {
            Op::suspense_wake_mutation(facts.select_suspense(selector), biased_wake_mutation(value))
        }
        21 if facts.has_dynamic_slots() => Op::dynamic(
            vnode,
            facts.select_dynamic_slot(vnode, selector),
            DynamicKind::Suspense {
                mode: SuspenseMode::Ready,
            },
        ),
        22 if facts.has_suspense() => Op::wake_suspense(facts.select_suspense(selector)),
        22 if facts.has_dynamic_slots() => Op::dynamic(
            vnode,
            facts.select_dynamic_slot(vnode, selector),
            DynamicKind::Suspense {
                mode: SuspenseMode::Ready,
            },
        ),
        23 if facts.has_suspense() => Op::wake_suspense_natural(facts.select_suspense(selector)),
        23 if facts.has_dynamic_slots() => Op::dynamic(
            vnode,
            facts.select_dynamic_slot(vnode, selector),
            DynamicKind::Suspense {
                mode: SuspenseMode::Ready,
            },
        ),
        24 if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::SetNode {
                node,
                kind: TemplateNodeKind::Element {
                    tag: value,
                    namespace: (selector & 1 == 0).then_some(selector),
                },
            },
        ),
        25 => Op::Rerender,
        _ => Op::template(
            vnode,
            TemplateEdit::SetNode {
                node,
                kind: TemplateNodeKind::Dynamic,
            },
        ),
    }
}

#[derive(Clone, Copy)]
struct FragmentShape {
    vnode: u8,
    slot: u8,
    len: usize,
    keyed: bool,
    suspense: Option<u8>,
    children: [Option<FragmentChildShape>; MAX_FRAGMENT_CHILDREN],
}

#[derive(Clone, Copy)]
struct AttrShape {
    vnode: u8,
    slot: u8,
    len: usize,
}

#[derive(Clone, Copy)]
struct FragmentChildShape {
    vnode: u8,
    index: u8,
    domless: bool,
}

#[derive(Clone, Copy)]
struct DynamicSlotShape {
    vnode: u8,
    slot: u8,
    nested: bool,
    domless: bool,
}

#[derive(Default)]
struct VNodeShape {
    roots: usize,
    nodes: usize,
    elements: Vec<ElementShape>,
    dynamic_slots: usize,
}

#[derive(Clone, Copy)]
struct ElementShape {
    children: usize,
    attrs: usize,
    can_insert_child: bool,
}

#[derive(Default)]
struct ModelFacts {
    vnodes: Vec<VNodeShape>,
    dynamic_slots: Vec<DynamicSlotShape>,
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
        let elements = vnode
            .template
            .element_paths()
            .into_iter()
            .map(|path| {
                let Some(TemplateNodeSpec::Element {
                    children, attrs, ..
                }) = template_node_at(&vnode.template.roots, &path)
                else {
                    return ElementShape {
                        children: 0,
                        attrs: 0,
                        can_insert_child: false,
                    };
                };
                ElementShape {
                    children: children.len(),
                    attrs: attrs.len(),
                    can_insert_child: children.len() < model::MAX_CHILDREN,
                }
            })
            .collect::<Vec<_>>();

        self.vnodes.push(VNodeShape {
            roots: vnode.template.roots.len(),
            nodes: vnode.template.node_paths().len(),
            elements,
            dynamic_slots: vnode.dynamics.len(),
        });

        for (slot, attrs) in vnode.attrs.iter().enumerate() {
            self.attrs.push(AttrShape {
                vnode: vnode_index,
                slot: slot as u8,
                len: attrs.len(),
            });
        }

        let dynamic_paths = collect_dynamic_slot_paths(&vnode.template.roots);
        for (slot, dynamic) in vnode.dynamics.iter().enumerate() {
            self.dynamic_slots.push(DynamicSlotShape {
                vnode: vnode_index,
                slot: slot as u8,
                nested: dynamic_paths
                    .get(slot)
                    .map(|path| path.len() > 1)
                    .unwrap_or(false),
                domless: !dynamic_creates_dom(dynamic),
            });

            match dynamic {
                DynamicSpec::Fragment(children) => {
                    let mut child_shapes = [None; MAX_FRAGMENT_CHILDREN];
                    for (index, child) in children.iter().enumerate() {
                        let child_vnode = self.collect_vnode(child, suspense);
                        if let Some(slot) = child_shapes.get_mut(index) {
                            *slot = Some(FragmentChildShape {
                                vnode: child_vnode,
                                index: index as u8,
                                domless: !vnode_creates_dom(child),
                            });
                        }
                    }
                    self.fragments.push(FragmentShape {
                        vnode: vnode_index,
                        slot: slot as u8,
                        len: children.len(),
                        keyed: children.first().and_then(|child| child.key).is_some(),
                        suspense,
                        children: child_shapes,
                    });
                }
                DynamicSpec::ComponentA(child) | DynamicSpec::ComponentB(child) => {
                    self.collect_vnode(child, suspense);
                }
                DynamicSpec::Suspense(suspense) => {
                    let suspense_index = self.suspense_count.min(u8::MAX as usize) as u8;
                    self.suspense_count += 1;
                    let child = self.collect_vnode(&suspense.child, Some(suspense_index));
                    self.suspense_child_vnodes.push(child);
                }
                DynamicSpec::Empty | DynamicSpec::Text(_) | DynamicSpec::Placeholder => {}
            }
        }

        vnode_index
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

    fn select_element_with_child_capacity(&self, vnode: u8, selector: u8) -> Option<u8> {
        let elements = &self.vnodes[vnode as usize].elements;
        let candidates = elements
            .iter()
            .enumerate()
            .filter(|(_, element)| element.can_insert_child)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        candidates
            .get(selector as usize % candidates.len().max(1))
            .copied()
            .map(|index| index as u8)
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

    fn select_dynamic_slot(&self, vnode: u8, selector: u8) -> u8 {
        select_bounded(selector, self.vnodes[vnode as usize].dynamic_slots)
    }

    fn has_dynamic_slots(&self) -> bool {
        self.vnodes.iter().any(|vnode| vnode.dynamic_slots > 0)
    }

    fn select_nested_domless_slot(&self, selector: u8) -> Option<DynamicSlotShape> {
        let slots = self
            .dynamic_slots
            .iter()
            .copied()
            .filter(|slot| slot.nested && slot.domless)
            .collect::<Vec<_>>();
        slots.get(selector as usize % slots.len().max(1)).copied()
    }

    fn select_fragment(&self, selector: u8) -> FragmentShape {
        if self.fragments.is_empty() {
            return FragmentShape {
                vnode: self.select_vnode(selector),
                slot: self.select_dynamic_slot(self.select_vnode(selector), selector),
                len: 0,
                keyed: false,
                suspense: None,
                children: [None; MAX_FRAGMENT_CHILDREN],
            };
        }
        self.fragments[selector as usize % self.fragments.len()]
    }

    fn select_keyed_fragment(&self, selector: u8, require_domless: bool) -> Option<FragmentShape> {
        self.select_fragment_matching(selector, |fragment| {
            fragment.keyed
                && fragment.len >= 2
                && (!require_domless || fragment.select_domless_child(selector).is_some())
        })
    }

    fn select_suspense_keyed_domless_fragment(&self, selector: u8) -> Option<FragmentShape> {
        self.select_fragment_matching(selector, |fragment| {
            fragment.suspense.is_some()
                && fragment.keyed
                && fragment.len >= 2
                && fragment.select_domless_child(selector).is_some()
        })
    }

    fn select_fragment_matching(
        &self,
        selector: u8,
        mut matches: impl FnMut(&FragmentShape) -> bool,
    ) -> Option<FragmentShape> {
        let fragments = self
            .fragments
            .iter()
            .copied()
            .filter(|fragment| matches(fragment))
            .collect::<Vec<_>>();
        fragments
            .get(selector as usize % fragments.len().max(1))
            .copied()
    }

    fn select_attr_slot(&self, selector: u8) -> AttrShape {
        if self.attrs.is_empty() {
            return AttrShape {
                vnode: self.select_vnode(selector),
                slot: 0,
                len: 0,
            };
        }
        self.attrs[selector as usize % self.attrs.len()]
    }

    fn has_attr_slots(&self) -> bool {
        !self.attrs.is_empty()
    }

    fn select_suspense(&self, selector: u8) -> u8 {
        select_bounded(selector, self.suspense_count)
    }

    fn has_suspense(&self) -> bool {
        self.suspense_count > 0
    }
}

impl FragmentShape {
    fn select_child_pair(&self, selector: u8) -> Vec<FragmentChildShape> {
        let children = self.children.iter().flatten().copied().collect::<Vec<_>>();
        if children.is_empty() {
            return Vec::new();
        }

        let first = selector as usize % children.len();
        let second = if children.len() > 1 {
            (first + 1) % children.len()
        } else {
            first
        };

        let mut selected = vec![children[first]];
        if second != first {
            selected.push(children[second]);
        }
        selected
    }

    fn select_domless_child(&self, selector: u8) -> Option<FragmentChildShape> {
        let children = self
            .children
            .iter()
            .flatten()
            .copied()
            .filter(|child| child.domless)
            .collect::<Vec<_>>();
        children
            .get(selector as usize % children.len().max(1))
            .copied()
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

fn collect_dynamic_slot_paths(roots: &[TemplateNodeSpec]) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    for (index, root) in roots.iter().enumerate() {
        collect_dynamic_slot_paths_from(root, vec![index], &mut out);
    }
    out
}

fn collect_dynamic_slot_paths_from(
    node: &TemplateNodeSpec,
    path: Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) {
    match node {
        TemplateNodeSpec::Dynamic => out.push(path),
        TemplateNodeSpec::Element { children, .. } => {
            for (index, child) in children.iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(index);
                collect_dynamic_slot_paths_from(child, child_path, out);
            }
        }
        TemplateNodeSpec::Text(_) => {}
    }
}

fn vnode_creates_dom(vnode: &VNodeSpec) -> bool {
    let mut dynamic_index = 0;
    vnode
        .template
        .roots
        .iter()
        .any(|root| template_node_creates_dom(root, vnode, &mut dynamic_index))
}

fn template_node_creates_dom(
    node: &TemplateNodeSpec,
    vnode: &VNodeSpec,
    dynamic_index: &mut usize,
) -> bool {
    match node {
        TemplateNodeSpec::Element { .. } | TemplateNodeSpec::Text(_) => true,
        TemplateNodeSpec::Dynamic => {
            let creates_dom = vnode
                .dynamics
                .get(*dynamic_index)
                .map(dynamic_creates_dom)
                .unwrap_or(false);
            *dynamic_index += 1;
            creates_dom
        }
    }
}

fn dynamic_creates_dom(dynamic: &DynamicSpec) -> bool {
    match dynamic {
        DynamicSpec::Empty => false,
        DynamicSpec::Fragment(children) => children.iter().any(vnode_creates_dom),
        DynamicSpec::ComponentA(child) | DynamicSpec::ComponentB(child) => vnode_creates_dom(child),
        DynamicSpec::Suspense(_) | DynamicSpec::Text(_) | DynamicSpec::Placeholder => true,
    }
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
        0 => TemplateNodeKind::Dynamic,
        1 => TemplateNodeKind::Text(value),
        _ => TemplateNodeKind::Element {
            tag: value,
            namespace: (value & 1 == 0).then_some(value.wrapping_add(1)),
        },
    }
}

fn biased_template_attr(value: u8) -> TemplateAttrSpec {
    if value & 1 == 0 {
        TemplateAttrSpec::Dynamic
    } else {
        TemplateAttrSpec::Static {
            name: value,
            value: value.wrapping_add(1),
            namespace: (value & 2 == 0).then_some(value.wrapping_add(2)),
        }
    }
}

fn biased_leaf_dynamic_kind(value: u8) -> DynamicKind {
    match value % 3 {
        0 => DynamicKind::Text(value),
        1 => DynamicKind::Placeholder,
        _ => DynamicKind::Empty,
    }
}

fn biased_suspense_mode(value: u8) -> SuspenseMode {
    match value % 3 {
        0 => SuspenseMode::Resolved,
        1 => SuspenseMode::Pending,
        _ => SuspenseMode::Ready,
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

fn biased_fragment_child_key(value: u8, len: usize, keyed: bool) -> Option<u8> {
    if keyed {
        Some(value.wrapping_add(len.min(u8::MAX as usize) as u8))
    } else {
        None
    }
}

fn optimized_attr(value: u8) -> AttrSpec {
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
        name: optimized_attr_name(&attr_value),
        namespace: None,
        value: attr_value,
        volatile: false,
    }
}

fn optimized_attr_name(value: &AttrValueSpec) -> u8 {
    match value {
        AttrValueSpec::Text(value)
        | AttrValueSpec::Float(value)
        | AttrValueSpec::Int(value)
        | AttrValueSpec::Any(value) => *value,
        AttrValueSpec::Bool(value) => u8::from(*value),
        AttrValueSpec::None => 0,
        AttrValueSpec::Listener => 1,
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

#[derive(Clone, Debug, PartialEq)]
pub struct FuzzFailure {
    step: usize,
    op: String,
    message: String,
}

impl FuzzFailure {
    pub fn step(&self) -> usize {
        self.step
    }

    pub fn op(&self) -> &str {
        &self.op
    }

    pub fn message(&self) -> &str {
        &self.message
    }
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
    writeln!(&mut report, "dioxus-vdom-fuzz failure").unwrap();
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

pub fn format_panic_failure_report(
    case: &FuzzCase,
    active_step: Option<usize>,
    panic_message: &str,
) -> String {
    let step = active_step
        .filter(|step| *step < case.ops.len())
        .unwrap_or_else(|| case.ops.len().saturating_sub(1));
    let op = case
        .ops
        .get(step)
        .map_or_else(|| "<unknown>".to_string(), |op| format!("{op:?}"));
    let failure = FuzzFailure {
        step,
        op,
        message: format!("panic while applying operation: {panic_message}"),
    };

    format_failure_report(case, &failure)
}

pub fn decode_case(data: &[u8]) -> Option<FuzzCase> {
    let mut case = postcard::from_bytes::<FuzzCase>(data).ok()?;
    case.normalize();
    Some(case)
}

pub fn encode_case(case: &FuzzCase, data: &mut [u8], max_size: usize) -> Option<usize> {
    let size = max_size.min(data.len());
    let encoded = postcard::to_slice(case, &mut data[..size]).ok()?;
    Some(encoded.len())
}

pub fn encode_case_vec(case: &FuzzCase) -> Option<Vec<u8>> {
    postcard::to_allocvec(case).ok()
}

pub fn run_case(case: &FuzzCase) -> Result<(), FuzzFailure> {
    let mut state = Harness::fresh();
    let active_step = ActiveRunStepGuard::new();
    for (step, op) in case.ops.iter().enumerate() {
        active_step.set(step);
        apply_step(&mut state, op).map_err(|message| FuzzFailure {
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
    fn seed_case_roundtrips_and_replays() {
        let case = FuzzCase::seed();
        assert!(case.is_empty());
        let mut bytes = [0; 4096];
        let size = encode_case(&case, &mut bytes, 4096).unwrap();
        let decoded = decode_case(&bytes[..size]).unwrap();
        assert_eq!(case, decoded);
        run_case(&decoded).unwrap();
    }

    #[test]
    fn optimized_model_aware_ops_replay() {
        let model = Model::initial();
        for which in 0..OPTIMIZED_MUTATION_STRATEGIES {
            let op = optimized_model_aware_op(&model, which, which as u8, 128 + which as u8);
            run_case(&FuzzCase::new(vec![op])).unwrap();
        }
    }

    #[test]
    fn optimized_model_aware_ops_replay_after_prefix() {
        let prefix = vec![
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(0, 0, DynamicKind::Fragment),
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
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            ),
            Op::dynamic(
                1,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
        ];
        let model = replay_model_prefix(&prefix, prefix.len());
        for which in 0..OPTIMIZED_MUTATION_STRATEGIES {
            let mut ops = prefix.clone();
            ops.push(optimized_model_aware_op(
                &model,
                which,
                64 + which as u8,
                192 + which as u8,
            ));
            run_case(&FuzzCase::new(ops)).unwrap();
        }
    }

    #[test]
    fn export_seed_case_when_requested() {
        let Ok(path) = std::env::var("DIOXUS_VDOM_FUZZ_EXPORT_SEED") else {
            return;
        };

        let case = FuzzCase::seed();
        let encoded = encode_case_vec(&case).unwrap();
        std::fs::write(path, encoded).unwrap();
    }
}
