//! Reusable Dioxus VirtualDom fuzzing harness.
//!
//! The `cargo-fuzz` target feeds encoded [`FuzzCase`] values into this crate.
//! LibFuzzer owns coverage guidance and corpus management; this crate owns the
//! structured operation stream and renderer oracle.

mod cache;
mod harness;
mod lifecycle;
mod model;
mod ops;
mod reducer;
mod vdom;

use harness::{Harness, apply_step, print_ssr_diff_trace};
use model::{
    AttrSpec, AttrValueSpec, DynamicKind, DynamicSpec, FragmentKeyMode, Model, SuspenseMode,
    TemplateAttrSpec, TemplateNodeKind, TemplateNodeSpec, VNodeSpec, WakeMutationSpec,
};
use mutatis::{Candidates, DefaultMutate, Generate, Mutate, Result as MutatisResult};
use ops::{FragmentEdit, ListEdit, Op, TemplateEdit};
pub use reducer::{ReduceError, ReductionOptions, ReductionReport, ReductionStats, reduce_case};
use reducer::{random_multistep_shrink_case, simplified_ops};
use serde::{Deserialize, Serialize};
use std::{cell::Cell, fmt};

pub const MAX_STEPS: usize = 512;
const OPTIMIZED_BURST_LIMIT: usize = 6;

const OPTIMIZED_STRATEGIES: &[OptimizedStrategy] = &[
    OptimizedStrategy::SetSelectedNodeBiased,
    OptimizedStrategy::InsertRoot,
    OptimizedStrategy::RemoveOrMoveRoot,
    OptimizedStrategy::InsertChild,
    OptimizedStrategy::RemoveOrMoveChild,
    OptimizedStrategy::InsertTemplateAttr,
    OptimizedStrategy::RemoveOrMoveTemplateAttr,
    OptimizedStrategy::SetDynamicFragment,
    OptimizedStrategy::SetDynamicLeaf,
    OptimizedStrategy::SetDynamicComponent,
    OptimizedStrategy::SetFragmentKeyMode,
    OptimizedStrategy::EditFragmentChildren,
    OptimizedStrategy::EditDynamicAttrs,
    OptimizedStrategy::SetSuspenseMode,
    OptimizedStrategy::SetSuspenseWakeMutation,
    OptimizedStrategy::WakeSuspense,
    OptimizedStrategy::SetSelectedNodeElement,
    OptimizedStrategy::Rerender,
];

#[derive(Clone, Copy, Debug)]
enum OptimizedStrategy {
    SetSelectedNodeBiased,
    InsertRoot,
    RemoveOrMoveRoot,
    InsertChild,
    RemoveOrMoveChild,
    InsertTemplateAttr,
    RemoveOrMoveTemplateAttr,
    SetDynamicFragment,
    SetDynamicLeaf,
    SetDynamicComponent,
    SetFragmentKeyMode,
    EditFragmentChildren,
    EditDynamicAttrs,
    SetSuspenseMode,
    SetSuspenseWakeMutation,
    WakeSuspense,
    SetSelectedNodeElement,
    Rerender,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FuzzCase {
    ops: Vec<Op>,
}

impl FuzzCase {
    pub(crate) fn new(mut ops: Vec<Op>) -> Self {
        ops.truncate(MAX_STEPS);
        Self { ops }
    }

    pub fn normalize(&mut self) {
        self.ops.truncate(MAX_STEPS);
    }
}

impl Default for FuzzCase {
    fn default() -> Self {
        Self::new(Vec::new())
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
            candidates.mutation_group(OPTIMIZED_STRATEGIES.len() as u32, |context, which| {
                let strategy = OPTIMIZED_STRATEGIES[which as usize];
                insert_optimized_model_aware_ops(context, case, strategy);
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
    strategy: OptimizedStrategy,
) {
    let index = context.rng().gen_index(case.ops.len() + 1).unwrap();
    let model = replay_model_prefix(&case.ops, index);
    let selector = context.rng().gen_u8();
    let value = context.rng().gen_u8();
    let op = optimized_model_aware_op(&model, strategy, selector, value);

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
    strategy: OptimizedStrategy,
) {
    insert_optimized_model_aware_op(context, case, strategy);

    let burst_len = context.rng().gen_index(OPTIMIZED_BURST_LIMIT).unwrap_or(0);
    for _ in 0..burst_len {
        let strategy = OPTIMIZED_STRATEGIES[context
            .rng()
            .gen_index(OPTIMIZED_STRATEGIES.len())
            .unwrap_or(0)];
        insert_optimized_model_aware_op(context, case, strategy);
    }
}

fn optimized_model_aware_op(
    model: &Model,
    strategy: OptimizedStrategy,
    selector: u8,
    value: u8,
) -> Op {
    let facts = ModelFacts::new(model);
    let vnode = facts.select_focus_vnode(selector, value);
    let node = facts.select_node(vnode, value);
    let element = facts.select_element(vnode, value);
    match strategy {
        OptimizedStrategy::SetSelectedNodeBiased if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::SetNode {
                node,
                kind: biased_template_node_kind(value),
            },
        ),
        OptimizedStrategy::InsertRoot if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::Roots {
                edit: ListEdit::Insert {
                    index: biased_index(value, facts.root_count(vnode)),
                    item: biased_template_node_kind(value),
                },
            },
        ),
        OptimizedStrategy::RemoveOrMoveRoot => Op::template(
            vnode,
            TemplateEdit::Roots {
                edit: remove_or_move_list_edit(facts.root_count(vnode), selector, value),
            },
        ),
        OptimizedStrategy::InsertChild if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::Children {
                element,
                edit: ListEdit::Insert {
                    index: biased_index(value, facts.child_count(vnode, element)),
                    item: biased_template_node_kind(value),
                },
            },
        ),
        OptimizedStrategy::RemoveOrMoveChild => Op::template(
            vnode,
            TemplateEdit::Children {
                element,
                edit: remove_or_move_list_edit(facts.child_count(vnode, element), selector, value),
            },
        ),
        OptimizedStrategy::InsertTemplateAttr if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::Attrs {
                element,
                edit: ListEdit::Insert {
                    index: biased_index(value, facts.template_attr_count(vnode, element)),
                    item: biased_template_attr(value),
                },
            },
        ),
        OptimizedStrategy::RemoveOrMoveTemplateAttr => Op::template(
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
        OptimizedStrategy::SetDynamicFragment if facts.has_dynamic_slots() => {
            dynamic_slot_op(&facts, vnode, selector, DynamicKind::Fragment)
        }
        OptimizedStrategy::SetDynamicLeaf if facts.has_dynamic_slots() => {
            dynamic_slot_op(&facts, vnode, selector, biased_leaf_dynamic_kind(value))
        }
        OptimizedStrategy::SetDynamicComponent if facts.has_dynamic_slots() => dynamic_slot_op(
            &facts,
            vnode,
            selector,
            if value & 1 == 0 {
                DynamicKind::ComponentA
            } else {
                DynamicKind::ComponentB
            },
        ),
        OptimizedStrategy::SetFragmentKeyMode if facts.has_dynamic_slots() => {
            let fragment = facts
                .select_fragment(selector)
                .unwrap_or_else(|| facts.fragment_prerequisite(selector));
            Op::fragment(
                fragment.vnode,
                fragment.slot,
                FragmentEdit::KeyMode(biased_fragment_key_mode(value)),
            )
        }
        OptimizedStrategy::EditFragmentChildren if facts.has_dynamic_slots() => {
            edit_fragment_children_op(&facts, model.can_grow(), selector, value)
        }
        OptimizedStrategy::EditDynamicAttrs => {
            edit_dynamic_attrs_op(&facts, model.can_grow(), vnode, element, selector, value)
        }
        OptimizedStrategy::SetSuspenseMode if facts.has_suspense() => {
            Op::suspense(facts.select_suspense(selector), biased_suspense_mode(value))
        }
        OptimizedStrategy::SetSuspenseMode if facts.has_dynamic_slots() => dynamic_slot_op(
            &facts,
            vnode,
            selector,
            DynamicKind::Suspense {
                mode: biased_suspense_mode(value),
            },
        ),
        OptimizedStrategy::SetSuspenseWakeMutation if facts.has_suspense() => {
            Op::suspense_wake_mutation(facts.select_suspense(selector), biased_wake_mutation(value))
        }
        OptimizedStrategy::SetSuspenseWakeMutation if facts.has_dynamic_slots() => {
            ready_suspense_slot_op(&facts, vnode, selector)
        }
        OptimizedStrategy::WakeSuspense if facts.has_suspense() => {
            Op::wake_suspense(facts.select_suspense(selector))
        }
        OptimizedStrategy::WakeSuspense if facts.has_dynamic_slots() => {
            ready_suspense_slot_op(&facts, vnode, selector)
        }
        OptimizedStrategy::SetSelectedNodeElement if model.can_grow() => Op::template(
            vnode,
            TemplateEdit::SetNode {
                node,
                kind: TemplateNodeKind::Element {
                    tag: value,
                    namespace: (selector & 1 == 0).then_some(selector),
                },
            },
        ),
        OptimizedStrategy::Rerender => Op::Rerender,
        _ => Op::template(
            vnode,
            TemplateEdit::SetNode {
                node,
                kind: TemplateNodeKind::Dynamic,
            },
        ),
    }
}

fn dynamic_slot_op(facts: &ModelFacts, vnode: u8, selector: u8, kind: DynamicKind) -> Op {
    Op::dynamic(vnode, facts.select_dynamic_slot(vnode, selector), kind)
}

fn ready_suspense_slot_op(facts: &ModelFacts, vnode: u8, selector: u8) -> Op {
    dynamic_slot_op(
        facts,
        vnode,
        selector,
        DynamicKind::Suspense {
            mode: SuspenseMode::Ready { wakes: 0 },
        },
    )
}

fn edit_fragment_children_op(facts: &ModelFacts, can_grow: bool, selector: u8, value: u8) -> Op {
    let fragment = facts
        .select_fragment(selector)
        .unwrap_or_else(|| facts.fragment_prerequisite(selector));
    let edit = match value % 3 {
        0 if can_grow => ListEdit::Insert {
            index: biased_index(value, fragment.len),
            item: biased_fragment_child_key(value, fragment.len, fragment.keyed),
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
            item: biased_fragment_child_key(value, fragment.len, fragment.keyed),
        },
        _ => ListEdit::Remove { index: 0 },
    };

    Op::fragment(fragment.vnode, fragment.slot, FragmentEdit::Children(edit))
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
            item: optimized_attr(value),
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
            item: optimized_attr(value),
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
                item: TemplateAttrSpec::Dynamic,
            },
        },
    )
}

#[derive(Clone, Copy)]
struct FragmentShape {
    vnode: u8,
    slot: u8,
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
    dynamic_slots: usize,
}

#[derive(Clone, Copy)]
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
                    };
                };
                ElementShape {
                    children: children.len(),
                    attrs: attrs.len(),
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

        for (slot, dynamic) in vnode.dynamics.iter().enumerate() {
            match dynamic {
                DynamicSpec::Fragment(children) => {
                    for child in children {
                        self.collect_vnode(child, suspense);
                    }
                    self.fragments.push(FragmentShape {
                        vnode: vnode_index,
                        slot: slot as u8,
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

    fn select_fragment(&self, selector: u8) -> Option<FragmentShape> {
        self.fragments
            .get(selector as usize % self.fragments.len().max(1))
            .copied()
    }

    fn fragment_prerequisite(&self, selector: u8) -> FragmentShape {
        let vnode = self.select_vnode(selector);
        FragmentShape {
            vnode,
            slot: self.select_dynamic_slot(vnode, selector),
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
        _ => SuspenseMode::Ready { wakes: value / 3 },
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
    fn empty_case_roundtrips_and_replays() {
        let case = FuzzCase::default();
        let mut bytes = [0; 4096];
        let size = encode_case(&case, &mut bytes, 4096).unwrap();
        let decoded = decode_case(&bytes[..size]).unwrap();
        assert_eq!(case, decoded);
        run_case(&decoded).unwrap();
    }

    #[test]
    fn optimized_model_aware_op_replays() {
        let model = Model::initial();
        for (index, strategy) in OPTIMIZED_STRATEGIES.iter().copied().enumerate() {
            let op = optimized_model_aware_op(&model, strategy, index as u8, 128 + index as u8);
            run_case(&FuzzCase::new(vec![op])).unwrap();
        }
    }

    #[test]
    fn optimized_model_aware_op_replays_after_prefix() {
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
                    mode: SuspenseMode::Ready { wakes: 0 },
                },
            ),
        ];
        let model = replay_model_prefix(&prefix, prefix.len());
        for (index, strategy) in OPTIMIZED_STRATEGIES.iter().copied().enumerate() {
            let mut ops = prefix.clone();
            ops.push(optimized_model_aware_op(
                &model,
                strategy,
                64 + index as u8,
                192 + index as u8,
            ));
            run_case(&FuzzCase::new(ops)).unwrap();
        }
    }
}
