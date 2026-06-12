//! Structure-aware mutation of [`FuzzCase`] operation streams.
//!
//! Mutation happens on two levels:
//!
//! - The derived mutatis mutators tweak encoded [`Op`]s field by field.
//! - A table of model-aware [`OpStrategy`]s ([`OP_STRATEGIES`]) replays the
//!   ops leading up to a splice point, summarizes the resulting model as
//!   [`ModelFacts`], and inserts op sequences that target real vnodes,
//!   fragments, attribute slots, and suspense boundaries in that model.
//!   Strategies whose target structure is missing emit their own
//!   prerequisite ops first, so every strategy stays meaningful from any
//!   model state.

use crate::case::{FuzzCase, MAX_STEPS};
use crate::model::{
    ATTR_NAME_POOL_MASK, AttrSpec, AttrValueSpec, DynamicKind, DynamicSpec, FragmentKeyMode, Model,
    SuspenseMode, TemplateAttrSpec, TemplateNodeKind, TemplateNodeSpec, WakeMutationSpec, select,
};
use crate::ops::{
    EventBehaviorSpec, FragmentEdit, ListEdit, Op, TemplateEdit, apply_strategy_op_to_model,
};
use crate::reducer::{random_multistep_shrink_case, simplified_ops};
use mutatis::{Candidates, Generate, Mutate, Result as MutatisResult, Session};

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

        candidates.mutation_group(OP_STRATEGIES.len() as u32, |context, which| {
            splice_strategy_ops(context, case, which as usize);
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

fn replay_model_prefix(ops: &[Op], len: usize) -> Model {
    let mut model = Model::initial();
    for op in ops.iter().take(len) {
        apply_strategy_op_to_model(&mut model, op);
    }
    model
}

fn splice_strategy_ops(context: &mut mutatis::Context, case: &mut FuzzCase, which: usize) {
    let index = context.rng().gen_index(case.ops.len() + 1).unwrap();
    let model = replay_model_prefix(&case.ops, index);
    let facts = ModelFacts::new(&model);
    let cx = StrategyCx {
        model: &model,
        facts: &facts,
        selector: context.rng().gen_u8(),
        value: context.rng().gen_u8(),
    };
    let ops = (OP_STRATEGIES[which].generate)(&cx);
    for (offset, op) in ops.into_iter().enumerate() {
        if case.ops.len() < MAX_STEPS {
            case.ops.insert(index + offset, op);
        } else {
            let replace = (index + offset).min(case.ops.len() - 1);
            case.ops[replace] = op;
        }
    }
}

// --- The op space ------------------------------------------------------------

/// Context handed to each [`OpStrategy`]: the model state at the splice point
/// plus two random bytes that drive selector/value decisions.
struct StrategyCx<'a> {
    model: &'a Model,
    facts: &'a ModelFacts,
    selector: u8,
    value: u8,
}

impl StrategyCx<'_> {
    fn focus_vnode(&self) -> u8 {
        self.facts.select_focus_vnode(self.selector, self.value)
    }
}

struct OpStrategy {
    /// Name used by tests and debugging output.
    #[allow(dead_code)]
    name: &'static str,
    /// Build the op sequence to splice into the case at the chosen point.
    generate: fn(&StrategyCx) -> Vec<Op>,
}

macro_rules! strategies {
    ($($name:ident),* $(,)?) => {
        &[$(OpStrategy { name: stringify!($name), generate: $name }),*]
    };
}

/// Every model-aware op sequence the mutator can splice into a case, with
/// uniform selection weight.
static OP_STRATEGIES: &[OpStrategy] = strategies![
    set_template_node,
    insert_root,
    remove_or_move_root,
    insert_child,
    remove_or_move_child,
    insert_template_attr,
    remove_or_move_template_attr,
    set_dynamic_fragment,
    set_dynamic_leaf,
    set_dynamic_component,
    fragment_key_mode,
    edit_fragment_children,
    edit_dynamic_attrs,
    set_suspense_mode,
    set_suspense_wake_mutation,
    wake_suspense,
    fire_event,
    set_namespaced_element,
    alias_attr_collision,
    rerender,
    render_dirty,
    render_suspense_dirty,
];

fn set_template_node(cx: &StrategyCx) -> Vec<Op> {
    let vnode = cx.focus_vnode();
    let node = cx.facts.select_node(vnode, cx.value);
    let kind = if cx.model.can_grow() {
        biased_template_node_kind(cx.value)
    } else {
        TemplateNodeKind::Dynamic(biased_leaf_dynamic_kind(cx.value))
    };
    vec![Op::template(vnode, TemplateEdit::SetNode { node, kind })]
}

fn insert_root(cx: &StrategyCx) -> Vec<Op> {
    let vnode = cx.focus_vnode();
    let len = cx.facts.root_count(vnode);
    let edit = if cx.model.can_grow() {
        ListEdit::Insert {
            index: biased_index(cx.value, len),
            item: biased_template_node_kind(cx.value),
        }
    } else {
        remove_or_move_list_edit(len, cx.selector, cx.value)
    };
    vec![Op::template(vnode, TemplateEdit::Roots { edit })]
}

fn remove_or_move_root(cx: &StrategyCx) -> Vec<Op> {
    let vnode = cx.focus_vnode();
    vec![Op::template(
        vnode,
        TemplateEdit::Roots {
            edit: remove_or_move_list_edit(cx.facts.root_count(vnode), cx.selector, cx.value),
        },
    )]
}

fn insert_child(cx: &StrategyCx) -> Vec<Op> {
    let vnode = cx.focus_vnode();
    let element = cx.facts.select_element(vnode, cx.value);
    let len = cx.facts.child_count(vnode, element);
    let edit = if cx.model.can_grow() {
        ListEdit::Insert {
            index: biased_index(cx.value, len),
            item: biased_template_node_kind(cx.value),
        }
    } else {
        remove_or_move_list_edit(len, cx.selector, cx.value)
    };
    vec![Op::template(
        vnode,
        TemplateEdit::Children { element, edit },
    )]
}

fn remove_or_move_child(cx: &StrategyCx) -> Vec<Op> {
    let vnode = cx.focus_vnode();
    let element = cx.facts.select_element(vnode, cx.value);
    vec![Op::template(
        vnode,
        TemplateEdit::Children {
            element,
            edit: remove_or_move_list_edit(
                cx.facts.child_count(vnode, element),
                cx.selector,
                cx.value,
            ),
        },
    )]
}

fn insert_template_attr(cx: &StrategyCx) -> Vec<Op> {
    let vnode = cx.focus_vnode();
    let element = cx.facts.select_element(vnode, cx.value);
    let len = cx.facts.template_attr_count(vnode, element);
    let edit = if cx.model.can_grow() {
        ListEdit::Insert {
            index: biased_index(cx.value, len),
            item: biased_template_attr(cx.value),
        }
    } else {
        remove_or_move_list_edit(len, cx.selector, cx.value)
    };
    vec![Op::template(vnode, TemplateEdit::Attrs { element, edit })]
}

fn remove_or_move_template_attr(cx: &StrategyCx) -> Vec<Op> {
    let vnode = cx.focus_vnode();
    let element = cx.facts.select_element(vnode, cx.value);
    vec![Op::template(
        vnode,
        TemplateEdit::Attrs {
            element,
            edit: remove_or_move_list_edit(
                cx.facts.template_attr_count(vnode, element),
                cx.selector,
                cx.value,
            ),
        },
    )]
}

fn set_dynamic_fragment(cx: &StrategyCx) -> Vec<Op> {
    vec![dynamic_node_op(cx, biased_fragment_dynamic_kind(cx.value))]
}

fn set_dynamic_leaf(cx: &StrategyCx) -> Vec<Op> {
    vec![dynamic_node_op(cx, biased_leaf_dynamic_kind(cx.value))]
}

fn set_dynamic_component(cx: &StrategyCx) -> Vec<Op> {
    let kind = if cx.value & 1 == 0 {
        DynamicKind::ComponentA
    } else {
        DynamicKind::ComponentB
    };
    vec![dynamic_node_op(cx, kind)]
}

fn fragment_key_mode(cx: &StrategyCx) -> Vec<Op> {
    let (mut ops, fragment) = select_or_create_fragment(cx);
    ops.push(Op::fragment(
        fragment.vnode,
        fragment.node,
        FragmentEdit::KeyMode(biased_fragment_key_mode(cx.value)),
    ));
    ops
}

fn edit_fragment_children(cx: &StrategyCx) -> Vec<Op> {
    let (mut ops, fragment) = select_or_create_fragment(cx);
    let can_grow = cx.model.can_grow();
    let edit = match cx.value % 3 {
        0 if can_grow => ListEdit::Insert {
            index: biased_index(cx.value, fragment.len),
            item: fragment_insert_key(fragment, cx.value),
        },
        1 if fragment.len > 0 => ListEdit::Remove {
            index: biased_existing_index(cx.value, fragment.len),
        },
        2 if fragment.len >= 2 => ListEdit::Move {
            from: biased_existing_index(cx.selector, fragment.len),
            to: biased_index(cx.value, fragment.len),
        },
        _ if can_grow => ListEdit::Insert {
            index: 0,
            item: fragment_insert_key(fragment, cx.value),
        },
        _ => ListEdit::Remove { index: 0 },
    };
    ops.push(Op::fragment(
        fragment.vnode,
        fragment.node,
        FragmentEdit::Children(edit),
    ));
    ops
}

/// Pick an existing fragment, or target a dynamic slot that the fragment
/// edit will convert in place, creating one first when the model has none.
fn select_or_create_fragment(cx: &StrategyCx) -> (Vec<Op>, FragmentShape) {
    if let Some(fragment) = cx.facts.select_fragment(cx.selector) {
        return (Vec::new(), fragment);
    }

    let fragment = cx.facts.fragment_prerequisite(cx.selector);
    let mut ops = Vec::new();
    if !cx.facts.has_dynamic_nodes() {
        ops.push(Op::dynamic(
            fragment.vnode,
            cx.facts.select_dynamic_node(fragment.vnode, cx.selector),
            DynamicKind::Fragment {
                children: 0,
                key_base: (cx.value & 4 != 0).then_some(cx.value),
            },
        ));
    }
    (ops, fragment)
}

fn edit_dynamic_attrs(cx: &StrategyCx) -> Vec<Op> {
    if let Some(attr) = cx.facts.select_attr_slot(cx.selector) {
        let edit = match cx.value % 3 {
            1 if attr.len > 0 => ListEdit::Remove {
                index: biased_existing_index(cx.value, attr.len),
            },
            2 if attr.len >= 2 => ListEdit::Move {
                from: biased_existing_index(cx.selector, attr.len),
                to: biased_index(cx.value, attr.len),
            },
            _ => ListEdit::Insert {
                index: biased_index(cx.value, attr.len),
                item: biased_attr(cx.value),
            },
        };
        return vec![Op::dynamic_attrs(attr.vnode, attr.slot, edit)];
    }

    // No dynamic attribute slot exists anywhere; create one and edit it.
    let vnode = cx.focus_vnode();
    let element = cx.facts.select_element(vnode, cx.value);
    vec![
        insert_dynamic_attr_slot_op(cx, vnode, element),
        Op::dynamic_attrs(
            vnode,
            0,
            ListEdit::Insert {
                index: 0,
                item: biased_attr(cx.value),
            },
        ),
    ]
}

fn insert_dynamic_attr_slot_op(cx: &StrategyCx, vnode: u8, element: u8) -> Op {
    Op::template(
        vnode,
        TemplateEdit::Attrs {
            element,
            edit: ListEdit::Insert {
                index: biased_index(cx.value, cx.facts.template_attr_count(vnode, element)),
                item: TemplateAttrSpec::Dynamic(vec![biased_attr(cx.value)]),
            },
        },
    )
}

fn set_suspense_mode(cx: &StrategyCx) -> Vec<Op> {
    let mode = biased_suspense_mode(cx.value);
    if cx.facts.has_suspense() {
        vec![Op::suspense(cx.facts.select_suspense(cx.selector), mode)]
    } else {
        vec![dynamic_node_op(cx, DynamicKind::Suspense { mode })]
    }
}

fn set_suspense_wake_mutation(cx: &StrategyCx) -> Vec<Op> {
    let mutation = biased_wake_mutation(cx.value);
    if cx.facts.has_suspense() {
        vec![Op::suspense_wake_mutation(
            cx.facts.select_suspense(cx.selector),
            mutation,
        )]
    } else {
        vec![
            ready_suspense_node_op(cx),
            Op::suspense_wake_mutation(0, mutation),
        ]
    }
}

fn wake_suspense(cx: &StrategyCx) -> Vec<Op> {
    if cx.facts.has_suspense() {
        vec![Op::wake_suspense(cx.facts.select_suspense(cx.selector))]
    } else {
        // Create a ready boundary, render so its task registers a waker,
        // then wake it.
        vec![
            ready_suspense_node_op(cx),
            Op::Rerender,
            Op::wake_suspense(0),
        ]
    }
}

fn ready_suspense_node_op(cx: &StrategyCx) -> Op {
    dynamic_node_op(
        cx,
        DynamicKind::Suspense {
            mode: SuspenseMode::Ready { wake_after: 0 },
        },
    )
}

fn fire_event(cx: &StrategyCx) -> Vec<Op> {
    vec![Op::fire_event(
        cx.selector,
        biased_event_behavior(cx.selector, cx.value),
    )]
}

fn set_namespaced_element(cx: &StrategyCx) -> Vec<Op> {
    let vnode = cx.focus_vnode();
    let node = cx.facts.select_node(vnode, cx.value);
    let kind = if cx.model.can_grow() {
        TemplateNodeKind::Element {
            tag: cx.value,
            namespace: (cx.selector & 1 == 0).then_some(cx.selector),
        }
    } else {
        TemplateNodeKind::Dynamic(biased_leaf_dynamic_kind(cx.value))
    };
    vec![Op::template(vnode, TemplateEdit::SetNode { node, kind })]
}

/// Build the alias-then-remove sequence that drives
/// `diff_attributes::remove_attribute_or_write_fallback`.
///
/// The first op inserts a *static* template attribute on an element with the
/// same resolved name as one of its existing dynamic attributes; the second
/// removes the dynamic side. After the next `Rerender` (the mutator splices
/// rerenders on its own) the diff sees the dynamic attribute disappear while
/// the static one stays, falling back to the static value. When the model
/// has no eligible dynamic attribute, the whole collision is bootstrapped
/// from scratch.
fn alias_attr_collision(cx: &StrategyCx) -> Vec<Op> {
    let candidates = cx.facts.collision_candidates();
    if let Some(pick) = select(candidates, cx.selector) {
        let alias = Op::template(
            pick.vnode,
            TemplateEdit::Attrs {
                element: pick.element,
                edit: ListEdit::Insert {
                    index: biased_index(cx.value, pick.element_attr_count),
                    item: TemplateAttrSpec::Static {
                        name: pick.name,
                        value: cx.value.wrapping_add(1),
                        namespace: None,
                    },
                },
            },
        );
        let drop_dynamic = Op::dynamic_attrs(pick.vnode, pick.slot, ListEdit::Remove { index: 0 });
        return vec![alias, drop_dynamic];
    }

    // Bootstrap a collision from nothing: insert a dynamic attribute with a
    // pool name, alias it with a static attribute of the same name, render,
    // then drop the dynamic side and render again.
    let vnode = cx.focus_vnode();
    let element = cx.facts.select_element(vnode, cx.value);
    let name = cx.value & ATTR_NAME_POOL_MASK;
    let slot = cx.facts.attr_slot_count(vnode).min(u8::MAX as usize) as u8;
    let attr_count = cx.facts.template_attr_count(vnode, element);
    vec![
        Op::template(
            vnode,
            TemplateEdit::Attrs {
                element,
                edit: ListEdit::Insert {
                    index: biased_index(cx.value, attr_count),
                    item: TemplateAttrSpec::Dynamic(vec![AttrSpec {
                        name,
                        namespace: None,
                        value: AttrValueSpec::Text(cx.value),
                        volatile: false,
                    }]),
                },
            },
        ),
        Op::template(
            vnode,
            TemplateEdit::Attrs {
                element,
                edit: ListEdit::Insert {
                    index: 0,
                    item: TemplateAttrSpec::Static {
                        name,
                        value: cx.value.wrapping_add(1),
                        namespace: None,
                    },
                },
            },
        ),
        Op::Rerender,
        Op::dynamic_attrs(vnode, slot, ListEdit::Remove { index: 0 }),
        Op::Rerender,
    ]
}

fn rerender(_cx: &StrategyCx) -> Vec<Op> {
    vec![Op::Rerender]
}

fn render_dirty(_cx: &StrategyCx) -> Vec<Op> {
    vec![Op::RenderDirty]
}

fn render_suspense_dirty(_cx: &StrategyCx) -> Vec<Op> {
    vec![Op::RenderSuspenseDirty]
}

fn dynamic_node_op(cx: &StrategyCx, kind: DynamicKind) -> Op {
    let vnode = cx.focus_vnode();
    Op::dynamic(
        vnode,
        cx.facts.select_dynamic_node(vnode, cx.selector),
        kind,
    )
}

// --- Biased value generators -------------------------------------------------

fn biased_event_behavior(selector: u8, value: u8) -> EventBehaviorSpec {
    match value % 10 {
        0 => EventBehaviorSpec::Noop,
        1 => EventBehaviorSpec::DispatchNestedEvent { target: selector },
        2 => EventBehaviorSpec::ScheduleUpdate,
        3 => EventBehaviorSpec::ScheduleUpdateAny,
        4 => EventBehaviorSpec::NeedsUpdate,
        5 => EventBehaviorSpec::NeedsUpdateAny,
        6 => EventBehaviorSpec::ContextRoundTrip,
        7 => EventBehaviorSpec::RootContextRoundTrip,
        8 => EventBehaviorSpec::QueueEffect,
        _ => EventBehaviorSpec::SpawnIsomorphic,
    }
}

fn fragment_insert_key(fragment: FragmentShape, value: u8) -> Option<u8> {
    fragment
        .keyed
        .then_some(value.wrapping_add(fragment.len.min(u8::MAX as usize) as u8))
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
        4 => DynamicKind::Suspense {
            mode: biased_suspense_mode(value),
        },
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

// --- Model facts -------------------------------------------------------------

/// A shape summary of one replayed [`Model`], indexed with the same selector
/// numbering the ops use, so strategies can target structure that actually
/// exists. Built by [`ModelFacts::new`] from the model's canonical
/// [`VNodeSpec::visit`]-compatible pre-order.
#[derive(Default)]
struct ModelFacts {
    vnodes: Vec<VNodeShape>,
    fragments: Vec<FragmentShape>,
    attrs: Vec<AttrShape>,
    suspense_child_vnodes: Vec<u8>,
    suspense_count: usize,
}

#[derive(Clone, Copy)]
struct FragmentShape {
    vnode: u8,
    node: u8,
    len: usize,
    keyed: bool,
}

struct AttrShape {
    vnode: u8,
    slot: u8,
    len: usize,
    element: u8,
    element_attr_count: usize,
    /// Names of non-listener dynamic attributes in this slot that resolve to
    /// the same key as a static attribute of the same name byte.
    collision_names: Vec<u8>,
}

/// One dynamic attribute eligible for static/dynamic name aliasing.
#[derive(Clone, Copy)]
struct CollisionCandidate {
    vnode: u8,
    element: u8,
    element_attr_count: usize,
    slot: u8,
    name: u8,
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

impl ModelFacts {
    fn new(model: &Model) -> Self {
        let mut facts = Self::default();
        facts.collect_vnode(&model.root, None);
        facts
    }

    fn collect_vnode(&mut self, vnode: &crate::model::VNodeSpec, suspense: Option<u8>) -> u8 {
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

            let element = elements.len().min(u8::MAX as usize) as u8;
            for attr in attrs {
                if let TemplateAttrSpec::Dynamic(dynamic_attrs) = attr {
                    self.attrs.push(AttrShape {
                        vnode,
                        slot: (*slot).min(u8::MAX as usize) as u8,
                        len: dynamic_attrs.len(),
                        element,
                        element_attr_count: attrs.len(),
                        collision_names: dynamic_attrs
                            .iter()
                            .filter(|attr| {
                                // Skip listeners (their name space is
                                // disjoint from static attribute names) and
                                // any byte with the high bit set, which
                                // routes through listener naming regardless
                                // of the value variant.
                                !matches!(attr.value, AttrValueSpec::Listener)
                                    && attr.name & 0x80 == 0
                            })
                            .map(|attr| attr.name)
                            .collect(),
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

    /// Pick a vnode to edit, biased towards suspense children and non-root
    /// vnodes when they exist.
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

    /// An empty fragment shape pointing at a dynamic slot a fragment edit
    /// can convert, preferring vnodes that already have dynamic slots.
    fn fragment_prerequisite(&self, selector: u8) -> FragmentShape {
        let with_dynamics = self
            .vnodes
            .iter()
            .enumerate()
            .filter(|(_, shape)| !shape.dynamic_nodes.is_empty())
            .map(|(index, _)| index.min(u8::MAX as usize) as u8)
            .collect::<Vec<_>>();
        let vnode = select(with_dynamics, selector).unwrap_or_else(|| self.select_vnode(selector));
        let vnode_shape = &self.vnodes[vnode as usize];
        FragmentShape {
            vnode,
            node: select_bounded(selector, vnode_shape.dynamic_nodes.len()),
            len: 0,
            keyed: false,
        }
    }

    fn select_attr_slot(&self, selector: u8) -> Option<&AttrShape> {
        self.attrs.get(selector as usize % self.attrs.len().max(1))
    }

    fn attr_slot_count(&self, vnode: u8) -> usize {
        self.attrs.iter().filter(|attr| attr.vnode == vnode).count()
    }

    fn collision_candidates(&self) -> Vec<CollisionCandidate> {
        self.attrs
            .iter()
            .flat_map(|attr| {
                attr.collision_names.iter().map(|&name| CollisionCandidate {
                    vnode: attr.vnode,
                    element: attr.element,
                    element_attr_count: attr.element_attr_count,
                    slot: attr.slot,
                    name,
                })
            })
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case::run_case;
    use crate::model::ModelVisit;

    fn strategy_index(name: &str) -> usize {
        OP_STRATEGIES
            .iter()
            .position(|strategy| strategy.name == name)
            .unwrap_or_else(|| panic!("unknown strategy {name:?}"))
    }

    fn strategy_ops(model: &Model, which: usize, selector: u8, value: u8) -> Vec<Op> {
        let facts = ModelFacts::new(model);
        let cx = StrategyCx {
            model,
            facts: &facts,
            selector,
            value,
        };
        (OP_STRATEGIES[which].generate)(&cx)
    }

    fn first_dynamic(model: &Model) -> Option<&DynamicSpec> {
        let mut found = None;
        model.root.visit(&mut |visit, _| {
            if let ModelVisit::Dynamic(dynamic) = visit {
                if found.is_none() {
                    found = Some(dynamic);
                }
            }
        });
        found
    }

    #[test]
    fn every_strategy_replays_from_initial_model() {
        for (which, strategy) in OP_STRATEGIES.iter().enumerate() {
            let model = Model::initial();
            let ops = strategy_ops(&model, which, which as u8, 128 + which as u8);
            run_case(&FuzzCase::new(ops))
                .unwrap_or_else(|failure| panic!("strategy {:?} failed: {failure}", strategy.name));
        }
    }

    #[test]
    fn dynamic_strategies_from_initial_model_are_meaningful() {
        let dynamic_cases = [
            ("set_dynamic_fragment", 1),
            ("set_dynamic_leaf", 3),
            ("set_dynamic_component", 4),
            ("set_suspense_mode", 5),
            ("set_suspense_wake_mutation", 6),
            ("wake_suspense", 7),
        ];

        for (name, value) in dynamic_cases {
            let mut model = Model::initial();
            for op in strategy_ops(&model.clone(), strategy_index(name), 0, value) {
                apply_strategy_op_to_model(&mut model, &op);
            }
            let dynamic =
                first_dynamic(&model).unwrap_or_else(|| panic!("expected dynamic for {name}"));
            assert!(
                !matches!(dynamic, DynamicSpec::Empty),
                "expected non-empty dynamic for {name}"
            );
        }

        let mut model = Model::initial();
        for op in strategy_ops(&model.clone(), strategy_index("edit_dynamic_attrs"), 0, 9) {
            apply_strategy_op_to_model(&mut model, &op);
        }
        let attr_lists = model.root.template.dynamic_attr_lists();
        let attrs = attr_lists.first().expect("expected dynamic attr slot");
        assert!(!attrs.is_empty(), "expected non-empty dynamic attrs");
    }

    #[test]
    fn every_strategy_replays_after_prefix() {
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
            Op::dynamic(
                1,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
        ];
        let model = replay_model_prefix(&prefix, prefix.len());
        for (which, strategy) in OP_STRATEGIES.iter().enumerate() {
            let mut ops = prefix.clone();
            ops.extend(strategy_ops(
                &model,
                which,
                64 + which as u8,
                192 + which as u8,
            ));
            run_case(&FuzzCase::new(ops))
                .unwrap_or_else(|failure| panic!("strategy {:?} failed: {failure}", strategy.name));
        }
    }
}
