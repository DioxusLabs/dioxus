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
    AttrSpec, AttrValueSpec, DynamicKind, DynamicSpec, FragmentKeyMode, Model, SuspenseMode,
    TemplateAttrSpec, TemplateNodeKind, TemplateNodeSpec, VNodeSpec, WakeMutationSpec,
};
use mutatis::{Candidates, DefaultMutate, Generate, Mutate, Result as MutatisResult};
use ops::{FragmentEdit, ListEdit, Op, TemplateEdit};
pub use reducer::{ReduceError, ReductionOptions, ReductionReport, ReductionStats, reduce_case};
use reducer::{random_multistep_shrink_case, simplified_ops};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const MAX_STEPS: usize = 512;
const OPTIMIZED_MUTATION_STRATEGIES: u32 = 26;
const OPTIMIZED_BURST_LIMIT: usize = 6;

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
        facts.collect_vnode(&model.root);
        facts.suspense_count = model.root.suspense_count();
        facts
    }

    fn collect_vnode(&mut self, vnode: &VNodeSpec) -> u8 {
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
            if let DynamicSpec::Fragment(children) = dynamic {
                self.fragments.push(FragmentShape {
                    vnode: vnode_index,
                    slot: slot as u8,
                    len: children.len(),
                    keyed: children.first().and_then(|child| child.key).is_some(),
                });
            }
            collect_dynamic_vnodes(dynamic, self);
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

    fn select_fragment(&self, selector: u8) -> FragmentShape {
        if self.fragments.is_empty() {
            return FragmentShape {
                vnode: self.select_vnode(selector),
                slot: self.select_dynamic_slot(self.select_vnode(selector), selector),
                len: 0,
                keyed: false,
            };
        }
        self.fragments[selector as usize % self.fragments.len()]
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

fn collect_dynamic_vnodes(dynamic: &DynamicSpec, facts: &mut ModelFacts) {
    match dynamic {
        DynamicSpec::Fragment(children) => {
            for child in children {
                facts.collect_vnode(child);
            }
        }
        DynamicSpec::ComponentA(child) | DynamicSpec::ComponentB(child) => {
            facts.collect_vnode(child);
        }
        DynamicSpec::Suspense(suspense) => {
            let child = facts.collect_vnode(&suspense.child);
            facts.suspense_child_vnodes.push(child);
        }
        DynamicSpec::Empty | DynamicSpec::Text(_) | DynamicSpec::Placeholder => {}
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
    const CONTEXT: usize = 6;

    let mut report = String::new();
    let summary = failure.message.lines().next().unwrap_or(&failure.message);
    let (start, end) = trace_bounds(case.ops.len(), failure.step);

    use fmt::Write;
    writeln!(&mut report, "dioxus-vdom-fuzz failure").unwrap();
    writeln!(&mut report, "decoded operations: {}", case.ops.len()).unwrap();
    writeln!(&mut report, "failed at step: {}", failure.step).unwrap();
    writeln!(&mut report, "failing op: {}", failure.op).unwrap();
    writeln!(&mut report, "summary: {summary}").unwrap();
    writeln!(&mut report).unwrap();
    writeln!(&mut report, "operation window:").unwrap();
    if start > 0 {
        writeln!(&mut report, "  ... {} earlier ops omitted", start).unwrap();
    }
    for (index, op) in case.ops.iter().enumerate().take(end).skip(start) {
        let marker = if index == failure.step { ">>" } else { "  " };
        writeln!(&mut report, "{marker} {index:03}: {op:?}").unwrap();
    }
    if end < case.ops.len() {
        writeln!(
            &mut report,
            "  ... {} later ops omitted",
            case.ops.len() - end
        )
        .unwrap();
    }
    writeln!(&mut report).unwrap();
    writeln!(&mut report, "full error:").unwrap();
    for line in failure.message.lines() {
        writeln!(&mut report, "  {line}").unwrap();
    }

    fn trace_bounds(ops_len: usize, failing_step: usize) -> (usize, usize) {
        if ops_len <= CONTEXT * 4 {
            return (0, ops_len);
        }

        (
            failing_step.saturating_sub(CONTEXT),
            (failing_step + CONTEXT + 1).min(ops_len),
        )
    }

    report
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
    for (step, op) in case.ops.iter().enumerate() {
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
