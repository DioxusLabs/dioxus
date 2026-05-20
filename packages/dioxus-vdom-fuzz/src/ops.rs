use crate::model::*;
use mutatis::{Candidates, DefaultMutate, Generate, Mutate, Result as MutatisResult};
use serde::{Deserialize, Serialize};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll, Waker},
};

// ---------- Structured seed operation generation --------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IteratorScenario {
    BranchSweep,
    UnkeyedAppend,
    UnkeyedRemove,
    KeyedPrepend,
    KeyedAppend,
    KeyedMiddleInsert,
    KeyedMiddleRemove,
    KeyedReplaceAll,
    KeyedMoveNearFront,
    KeyedMoveFirstToEnd,
    NestedDomlessMove,
    PortalRetarget,
    LargeTemplateHashStress,
}

impl IteratorScenario {
    pub(crate) const ALL: [Self; 13] = [
        Self::BranchSweep,
        Self::UnkeyedAppend,
        Self::UnkeyedRemove,
        Self::KeyedPrepend,
        Self::KeyedAppend,
        Self::KeyedMiddleInsert,
        Self::KeyedMiddleRemove,
        Self::KeyedReplaceAll,
        Self::KeyedMoveNearFront,
        Self::KeyedMoveFirstToEnd,
        Self::NestedDomlessMove,
        Self::PortalRetarget,
        Self::LargeTemplateHashStress,
    ];
}

pub(crate) fn iterator_scenario_ops(scenario: IteratorScenario, key_base: u8) -> Vec<Op> {
    match scenario {
        IteratorScenario::BranchSweep => branch_sweep_scenario(),
        IteratorScenario::UnkeyedAppend => {
            let mut ops = unkeyed_fragment_with_len(2);
            ops.push(Op::Rerender);
            ops.push(fragment_insert(2, None));
            ops.push(Op::Rerender);
            ops
        }
        IteratorScenario::UnkeyedRemove => {
            let mut ops = unkeyed_fragment_with_len(3);
            ops.push(Op::Rerender);
            ops.push(fragment_remove(1));
            ops.push(Op::Rerender);
            ops
        }
        IteratorScenario::KeyedPrepend => {
            let mut ops = keyed_fragment_with_len(key_base, 3);
            ops.push(Op::Rerender);
            ops.push(fragment_insert(0, Some(key_base.wrapping_add(16))));
            ops.push(Op::Rerender);
            ops
        }
        IteratorScenario::KeyedAppend => {
            let mut ops = keyed_fragment_with_len(key_base, 3);
            ops.push(Op::Rerender);
            ops.push(fragment_insert(3, Some(key_base.wrapping_add(3))));
            ops.push(Op::Rerender);
            ops
        }
        IteratorScenario::KeyedMiddleInsert => {
            let mut ops = keyed_fragment_with_len(key_base, 3);
            ops.push(Op::Rerender);
            ops.push(fragment_insert(1, Some(key_base.wrapping_add(16))));
            ops.push(Op::Rerender);
            ops
        }
        IteratorScenario::KeyedMiddleRemove => {
            let mut ops = keyed_fragment_with_len(key_base, 4);
            ops.push(Op::Rerender);
            ops.push(fragment_remove(1));
            ops.push(Op::Rerender);
            ops
        }
        IteratorScenario::KeyedReplaceAll => {
            let mut ops = keyed_fragment_with_len(key_base, 3);
            ops.push(Op::Rerender);
            ops.push(fragment_key_mode(FragmentKeyMode::Keyed {
                base: key_base.wrapping_add(32),
            }));
            ops.push(Op::Rerender);
            ops
        }
        IteratorScenario::KeyedMoveNearFront => {
            let mut ops = keyed_fragment_with_len(key_base, 4);
            ops.push(Op::Rerender);
            ops.push(fragment_move(1, 0));
            ops.push(Op::Rerender);
            ops
        }
        IteratorScenario::KeyedMoveFirstToEnd => {
            let mut ops = keyed_fragment_with_len(key_base, 4);
            ops.push(Op::Rerender);
            ops.push(fragment_move(0, 3));
            ops.push(Op::Rerender);
            ops
        }
        IteratorScenario::NestedDomlessMove => nested_domless_move_scenario(),
        IteratorScenario::PortalRetarget => portal_retarget_scenario(),
        IteratorScenario::LargeTemplateHashStress => large_template_hash_stress_scenario(),
    }
}

fn branch_sweep_scenario() -> Vec<Op> {
    let mut ops = unkeyed_fragment_with_len(2);

    ops.push(Op::Rerender);
    ops.push(fragment_insert(2, None));
    ops.push(Op::Rerender);
    ops.push(fragment_remove(1));
    ops.push(Op::Rerender);

    ops.push(fragment_key_mode(FragmentKeyMode::Keyed { base: 0 }));
    ops.push(Op::Rerender);

    ops.push(fragment_insert(0, Some(16)));
    ops.push(Op::Rerender);
    ops.push(fragment_insert(3, Some(17)));
    ops.push(Op::Rerender);
    ops.push(fragment_remove(1));
    ops.push(Op::Rerender);
    ops.push(fragment_insert(1, Some(18)));
    ops.push(Op::Rerender);

    ops.push(fragment_move(1, 0));
    ops.push(Op::Rerender);
    ops.push(fragment_move(0, 3));
    ops.push(Op::Rerender);

    ops.push(fragment_key_mode(FragmentKeyMode::Keyed { base: 64 }));
    ops.push(Op::Rerender);

    ops.push(fragment_remove(3));
    ops.push(fragment_move(2, 1));
    ops.push(fragment_insert(3, Some(80)));
    ops.push(Op::Rerender);

    ops
}

fn make_root_dynamic() -> Op {
    Op::Template {
        vnode: 0,
        edit: TemplateEdit::SetNode {
            node: 0,
            kind: TemplateNodeKind::Dynamic,
        },
    }
}

fn fragment_insert(index: u8, item: Option<u8>) -> Op {
    Op::Fragment {
        vnode: 0,
        slot: 0,
        edit: FragmentEdit::Children(ListEdit::Insert { index, item }),
    }
}

fn fragment_remove(index: u8) -> Op {
    Op::Fragment {
        vnode: 0,
        slot: 0,
        edit: FragmentEdit::Children(ListEdit::Remove { index }),
    }
}

fn fragment_move(from: u8, to: u8) -> Op {
    Op::Fragment {
        vnode: 0,
        slot: 0,
        edit: FragmentEdit::Children(ListEdit::Move { from, to }),
    }
}

fn fragment_key_mode(mode: FragmentKeyMode) -> Op {
    Op::Fragment {
        vnode: 0,
        slot: 0,
        edit: FragmentEdit::KeyMode(mode),
    }
}

fn unkeyed_fragment_with_len(len: u8) -> Vec<Op> {
    let mut ops = Vec::with_capacity(len as usize + 1);
    ops.push(make_root_dynamic());
    for index in 0..len {
        ops.push(fragment_insert(index, None));
    }
    ops
}

fn keyed_fragment_with_len(key_base: u8, len: u8) -> Vec<Op> {
    let mut ops = Vec::with_capacity(len as usize + 1);
    ops.push(make_root_dynamic());
    for index in 0..len {
        ops.push(fragment_insert(index, Some(key_base.wrapping_add(index))));
    }
    ops
}

fn nested_domless_move_scenario() -> Vec<Op> {
    vec![
        make_root_dynamic(),
        fragment_insert(0, None),
        fragment_insert(0, None),
        fragment_insert(0, None),
        fragment_key_mode(FragmentKeyMode::Keyed { base: 0 }),
        fragment_insert(0, None),
        Op::Template {
            vnode: 6,
            edit: TemplateEdit::SetNode {
                node: 0,
                kind: TemplateNodeKind::Dynamic,
            },
        },
        Op::Template {
            vnode: 7,
            edit: TemplateEdit::Children {
                element: 0,
                edit: ListEdit::Insert {
                    index: 0,
                    item: TemplateNodeKind::Dynamic,
                },
            },
        },
        fragment_insert(0, None),
        Op::Fragment {
            vnode: 177,
            slot: 0,
            edit: FragmentEdit::Children(ListEdit::Insert {
                index: 0,
                item: None,
            }),
        },
        Op::Rerender,
        Op::Dynamic {
            vnode: 2,
            slot: 0,
            kind: DynamicKind::ComponentA,
        },
        fragment_move(3, 2),
        Op::Rerender,
    ]
}

fn portal_retarget_scenario() -> Vec<Op> {
    vec![
        make_root_dynamic(),
        Op::Dynamic {
            vnode: 0,
            slot: 0,
            kind: DynamicKind::Portal {
                target: PortalTargetSpec::TargetA,
            },
        },
        Op::Template {
            vnode: 1,
            edit: TemplateEdit::SetNode {
                node: 0,
                kind: TemplateNodeKind::Dynamic,
            },
        },
        Op::Fragment {
            vnode: 1,
            slot: 0,
            edit: FragmentEdit::Children(ListEdit::Insert {
                index: 0,
                item: Some(0),
            }),
        },
        Op::Rerender,
        Op::Dynamic {
            vnode: 0,
            slot: 0,
            kind: DynamicKind::Portal {
                target: PortalTargetSpec::TargetB,
            },
        },
        Op::Rerender,
        Op::Dynamic {
            vnode: 0,
            slot: 0,
            kind: DynamicKind::Portal {
                target: PortalTargetSpec::Noop,
            },
        },
        Op::Rerender,
        Op::Dynamic {
            vnode: 0,
            slot: 0,
            kind: DynamicKind::Portal {
                target: PortalTargetSpec::TargetA,
            },
        },
        Op::Rerender,
    ]
}

fn large_template_hash_stress_scenario() -> Vec<Op> {
    let mut ops = Vec::new();
    for index in 0..12 {
        let shape = 0x00D1_0A00_0000_0000u64 ^ (index / 2);
        ops.push(Op::Template {
            vnode: 0,
            edit: TemplateEdit::Generated {
                seed: (shape << 8) | (index as u64 + 1),
                dynamic_nodes: 257 + (index / 2) as u16 * 19,
                dynamic_attrs: 257 + (index / 2) as u16 * 13,
            },
        });
        ops.push(Op::Rerender);
    }
    ops
}

// ---------- Model operations -----------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Mutate)]
pub(crate) enum Op {
    Rerender,
    WakeSuspense {
        suspense: u8,
    },
    WakeSuspenseNatural {
        suspense: u8,
    },
    Template {
        vnode: u8,
        edit: TemplateEdit,
    },
    Dynamic {
        vnode: u8,
        slot: u8,
        kind: DynamicKind,
    },
    DynamicAttrs {
        vnode: u8,
        slot: u8,
        edit: ListEdit<AttrSpec>,
    },
    Fragment {
        vnode: u8,
        slot: u8,
        edit: FragmentEdit,
    },
    Suspense {
        suspense: u8,
        mode: SuspenseMode,
    },
    SuspenseWakeMutation {
        suspense: u8,
        mutation: WakeMutationSpec,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Mutate)]
pub(crate) enum TemplateEdit {
    SetNode {
        node: u8,
        kind: TemplateNodeKind,
    },
    Roots {
        edit: ListEdit<TemplateNodeKind>,
    },
    Children {
        element: u8,
        edit: ListEdit<TemplateNodeKind>,
    },
    Attrs {
        element: u8,
        edit: ListEdit<TemplateAttrSpec>,
    },
    Generated {
        seed: u64,
        dynamic_nodes: u16,
        dynamic_attrs: u16,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Mutate)]
pub(crate) enum FragmentEdit {
    KeyMode(FragmentKeyMode),
    Children(ListEdit<Option<u8>>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum ListEdit<T> {
    Insert { index: u8, item: T },
    Remove { index: u8 },
    Move { from: u8, to: u8 },
}

#[derive(Clone, Debug)]
pub(crate) struct ListEditMutator<T, M> {
    item: M,
    _phantom: PhantomData<fn() -> T>,
}

impl<T, M> Default for ListEditMutator<T, M>
where
    M: Default,
{
    fn default() -> Self {
        Self {
            item: M::default(),
            _phantom: PhantomData,
        }
    }
}

impl<T> DefaultMutate for ListEdit<T>
where
    T: DefaultMutate,
    T::DefaultMutate: Generate<T>,
{
    type DefaultMutate = ListEditMutator<T, T::DefaultMutate>;
}

impl<T, M> Generate<ListEdit<T>> for ListEditMutator<T, M>
where
    M: Generate<T>,
{
    fn generate(&mut self, context: &mut mutatis::Context) -> MutatisResult<ListEdit<T>> {
        Ok(match context.rng().gen_index(3).unwrap() {
            0 => ListEdit::Insert {
                index: context.rng().gen_u8(),
                item: self.item.generate(context)?,
            },
            1 => ListEdit::Remove {
                index: context.rng().gen_u8(),
            },
            _ => ListEdit::Move {
                from: context.rng().gen_u8(),
                to: context.rng().gen_u8(),
            },
        })
    }
}

impl<T, M> Mutate<ListEdit<T>> for ListEditMutator<T, M>
where
    M: Generate<T> + Mutate<T>,
{
    fn mutate(
        &mut self,
        candidates: &mut Candidates<'_>,
        value: &mut ListEdit<T>,
    ) -> MutatisResult<()> {
        let replacement_count = if candidates.shrink() { 2 } else { 3 };
        candidates.mutation_group(replacement_count, |context, which| {
            *value = match which {
                0 => ListEdit::Remove {
                    index: context.rng().gen_u8(),
                },
                1 => ListEdit::Move {
                    from: context.rng().gen_u8(),
                    to: context.rng().gen_u8(),
                },
                _ => ListEdit::Insert {
                    index: context.rng().gen_u8(),
                    item: self.item.generate(context)?,
                },
            };
            Ok(())
        })?;

        match value {
            ListEdit::Insert { index, item } => {
                candidates.mutation(|context| {
                    *index = context.rng().gen_u8();
                    Ok(())
                })?;
                self.item.mutate(candidates, item)?;
            }
            ListEdit::Remove { index } => {
                candidates.mutation(|context| {
                    *index = context.rng().gen_u8();
                    Ok(())
                })?;
            }
            ListEdit::Move { from, to } => {
                candidates.mutation(|context| {
                    *from = context.rng().gen_u8();
                    Ok(())
                })?;
                candidates.mutation(|context| {
                    *to = context.rng().gen_u8();
                    Ok(())
                })?;
            }
        }

        Ok(())
    }
}

thread_local! {
    static MODEL: RefCell<Model> = RefCell::new(Model::initial());
    static SUSPENSE_READY_RELEASED: RefCell<Vec<SuspenseReadyKey>> = RefCell::new(Vec::new());
    static SUSPENSE_READY_WAKERS: RefCell<Vec<(SuspenseReadyKey, Waker)>> = RefCell::new(Vec::new());
    static REGISTER_SUSPENSE_READY_SENDERS: Cell<bool> = Cell::new(true);
}

pub(crate) fn read_model() -> Model {
    MODEL.with(|m| m.borrow().clone())
}

pub(crate) fn with_model<R>(f: impl FnOnce(&mut Model) -> R) -> R {
    MODEL.with(|m| f(&mut m.borrow_mut()))
}

fn suspense_ready_released(key: SuspenseReadyKey) -> bool {
    REGISTER_SUSPENSE_READY_SENDERS.with(|enabled| {
        enabled.get() && SUSPENSE_READY_RELEASED.with(|released| released.borrow().contains(&key))
    })
}

fn register_suspense_ready_waker(key: SuspenseReadyKey, waker: Waker) {
    REGISTER_SUSPENSE_READY_SENDERS.with(|enabled| {
        if enabled.get() {
            SUSPENSE_READY_WAKERS.with(|wakers| wakers.borrow_mut().push((key, waker)));
        }
    });
}

pub(crate) fn release_suspense_ready_task(key: SuspenseReadyKey) {
    SUSPENSE_READY_RELEASED.with(|released| {
        if !released.borrow().contains(&key) {
            released.borrow_mut().push(key);
        }
    });
    SUSPENSE_READY_WAKERS.with(|wakers| {
        let mut wakers = wakers.borrow_mut();
        let mut index = 0;
        while index < wakers.len() {
            if wakers[index].0 == key {
                let (_, waker) = wakers.swap_remove(index);
                waker.wake();
            } else {
                index += 1;
            }
        }
    });
}

pub(crate) fn selected_registered_ready_suspense_key(selector: u8) -> Option<SuspenseReadyKey> {
    let registered = SUSPENSE_READY_WAKERS.with(|wakers| {
        let mut keys = Vec::new();
        for (key, _) in wakers.borrow().iter() {
            if !keys.contains(key) {
                keys.push(*key);
            }
        }
        keys
    });

    let mut ready = Vec::new();
    read_model().root.collect_ready_suspense_keys(&mut ready);
    ready.retain(|key| registered.contains(key));
    select(ready, selector)
}

pub(crate) fn clear_suspense_ready_tasks() {
    SUSPENSE_READY_RELEASED.with(|released| released.borrow_mut().clear());
    SUSPENSE_READY_WAKERS.with(|wakers| wakers.borrow_mut().clear());
}

struct SuspenseReadyRegistrationGuard {
    previous: bool,
}

impl Drop for SuspenseReadyRegistrationGuard {
    fn drop(&mut self) {
        REGISTER_SUSPENSE_READY_SENDERS.with(|enabled| enabled.set(self.previous));
    }
}

pub(crate) fn without_suspense_ready_registration<R>(f: impl FnOnce() -> R) -> R {
    let _guard = REGISTER_SUSPENSE_READY_SENDERS.with(|enabled| {
        let previous = enabled.replace(false);
        SuspenseReadyRegistrationGuard { previous }
    });
    f()
}

pub(crate) struct SuspenseReadyFuture {
    pub(crate) key: SuspenseReadyKey,
}

impl Future for SuspenseReadyFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let key = self.key;
        if suspense_ready_released(key) {
            Poll::Ready(())
        } else {
            register_suspense_ready_waker(key, cx.waker().clone());
            Poll::Pending
        }
    }
}

pub(crate) fn apply_op_to_model(model: &mut Model, op: &Op) {
    if matches!(op, Op::Rerender) {
        return;
    }

    let can_grow = model.can_grow();
    match op {
        Op::Rerender => {}
        Op::WakeSuspense { suspense } | Op::WakeSuspenseNatural { suspense } => {
            if let Some(key) = model.selected_ready_suspense_key(*suspense) {
                model.resolve_ready_suspense(key);
            }
        }
        Op::Template { vnode, edit } => {
            let vnode = model.selected_vnode_mut(*vnode);
            apply_template_edit(vnode, edit, can_grow);
            vnode.normalize_in_place();
        }
        Op::Dynamic { vnode, slot, kind } => {
            let mut next_suspense_id = model.next_suspense_id;
            {
                let vnode = model.selected_vnode_mut(*vnode);
                if !vnode.dynamics.is_empty() {
                    let index = *slot as usize % vnode.dynamics.len();
                    if can_grow || matches!(kind, DynamicKind::Empty | DynamicKind::Text(_)) {
                        vnode.dynamics[index].set_kind(kind, &mut next_suspense_id);
                    }
                }
                vnode.normalize_in_place();
            }
            model.next_suspense_id = next_suspense_id;
        }
        Op::DynamicAttrs { vnode, slot, edit } => {
            let vnode = model.selected_vnode_mut(*vnode);
            if !vnode.attrs.is_empty() {
                let index = *slot as usize % vnode.attrs.len();
                apply_attr_list_edit(&mut vnode.attrs[index], edit);
                sort_attrs(index, &mut vnode.attrs[index]);
            }
            vnode.normalize_in_place();
        }
        Op::Fragment { vnode, slot, edit } => {
            let vnode = model.selected_vnode_mut(*vnode);
            apply_fragment_edit(vnode, *slot, edit, can_grow);
            vnode.normalize_in_place();
        }
        Op::Suspense { suspense, mode } => {
            model.set_selected_suspense_mode(*suspense, *mode);
        }
        Op::SuspenseWakeMutation { suspense, mutation } => {
            model.set_selected_suspense_wake_mutation(*suspense, *mutation);
        }
    }
}

pub(crate) fn apply_to_model(op: &Op) {
    with_model(|model| apply_op_to_model(model, op));
}

fn apply_template_edit(vnode: &mut VNodeSpec, edit: &TemplateEdit, can_grow: bool) {
    match edit {
        TemplateEdit::SetNode { node, kind } => {
            vnode.template.cache_key = None;
            if let Some(path) = select(vnode.template.node_paths(), *node) {
                if let Some(node) = vnode.template.node_mut(&path) {
                    node.set_kind(kind);
                }
            }
        }
        TemplateEdit::Roots { edit } => {
            vnode.template.cache_key = None;
            apply_template_node_list_edit(&mut vnode.template.roots, edit, 1, MAX_ROOTS, can_grow);
        }
        TemplateEdit::Children { element, edit } => {
            vnode.template.cache_key = None;
            if let Some(path) = select(vnode.template.element_paths(), *element) {
                if let Some(TemplateNodeSpec::Element { children, .. }) =
                    vnode.template.element_mut(&path)
                {
                    apply_template_node_list_edit(children, edit, 0, MAX_CHILDREN, can_grow);
                }
            }
        }
        TemplateEdit::Attrs { element, edit } => {
            vnode.template.cache_key = None;
            if let Some(path) = select(vnode.template.element_paths(), *element) {
                if let Some(TemplateNodeSpec::Element { attrs, .. }) =
                    vnode.template.element_mut(&path)
                {
                    apply_template_attr_list_edit(attrs, edit);
                }
            }
        }
        TemplateEdit::Generated {
            seed,
            dynamic_nodes,
            dynamic_attrs,
        } => {
            vnode.template = TemplateSpec::generated(*seed, *dynamic_nodes, *dynamic_attrs);
        }
    }
}

fn apply_fragment_edit(vnode: &mut VNodeSpec, slot: u8, edit: &FragmentEdit, can_grow: bool) {
    match edit {
        FragmentEdit::KeyMode(mode) => {
            if let Some(children) = selected_fragment_mut(vnode, slot) {
                apply_fragment_key_mode(children, mode);
            }
        }
        FragmentEdit::Children(ListEdit::Insert { index, item }) => {
            if can_grow {
                if let Some(children) = selected_fragment_mut(vnode, slot) {
                    insert_fragment_child(children, *index, *item);
                }
            }
        }
        FragmentEdit::Children(ListEdit::Remove { index }) => {
            if let Some(children) = selected_existing_fragment_mut(vnode, slot) {
                remove_selected(children, *index, 0);
            }
        }
        FragmentEdit::Children(ListEdit::Move { from, to }) => {
            if let Some(children) = selected_existing_fragment_mut(vnode, slot) {
                move_selected(children, *from, *to);
            }
        }
    }
}

fn apply_template_node_list_edit(
    nodes: &mut Vec<TemplateNodeSpec>,
    edit: &ListEdit<TemplateNodeKind>,
    min_len: usize,
    max_len: usize,
    can_grow: bool,
) {
    match edit {
        ListEdit::Insert { index, item } => {
            if can_grow && nodes.len() < max_len {
                let index = insert_index(nodes.len(), *index);
                nodes.insert(index, TemplateNodeSpec::from_kind(item));
            }
        }
        ListEdit::Remove { index } => {
            remove_selected(nodes, *index, min_len);
        }
        ListEdit::Move { from, to } => {
            move_selected(nodes, *from, *to);
        }
    }
}

fn apply_template_attr_list_edit(
    attrs: &mut Vec<TemplateAttrSpec>,
    edit: &ListEdit<TemplateAttrSpec>,
) {
    match edit {
        ListEdit::Insert { index, item } => {
            if attrs.len() < MAX_TEMPLATE_ATTRS {
                let index = insert_index(attrs.len(), *index);
                attrs.insert(index, item.clone());
            }
        }
        ListEdit::Remove { index } => {
            remove_selected(attrs, *index, 0);
        }
        ListEdit::Move { from, to } => {
            move_selected(attrs, *from, *to);
        }
    }
}

fn apply_attr_list_edit(attrs: &mut Vec<AttrSpec>, edit: &ListEdit<AttrSpec>) {
    match edit {
        ListEdit::Insert { index, item } => {
            if attrs.len() < MAX_DYNAMIC_ATTRS {
                let index = insert_index(attrs.len(), *index);
                attrs.insert(index, item.clone());
            }
        }
        ListEdit::Remove { index } => {
            remove_selected(attrs, *index, 0);
        }
        ListEdit::Move { from, to } => {
            move_selected(attrs, *from, *to);
        }
    }
}

fn insert_index(len: usize, selector: u8) -> usize {
    selector as usize % (len + 1)
}

fn remove_selected<T>(items: &mut Vec<T>, selector: u8, min_len: usize) {
    if items.len() <= min_len {
        return;
    }
    let index = selector as usize % items.len();
    items.remove(index);
}

fn move_selected<T>(items: &mut Vec<T>, from: u8, to: u8) {
    if items.len() <= 1 {
        return;
    }
    let from = from as usize % items.len();
    let item = items.remove(from);
    let to = to as usize % (items.len() + 1);
    items.insert(to, item);
}

fn selected_dynamic_mut(vnode: &mut VNodeSpec, selector: u8) -> Option<&mut DynamicSpec> {
    if vnode.dynamics.is_empty() {
        return None;
    }
    let index = selector as usize % vnode.dynamics.len();
    Some(&mut vnode.dynamics[index])
}

fn selected_fragment_mut(vnode: &mut VNodeSpec, selector: u8) -> Option<&mut Vec<VNodeSpec>> {
    let dynamic = selected_dynamic_mut(vnode, selector)?;
    if !matches!(dynamic, DynamicSpec::Fragment(_)) {
        *dynamic = DynamicSpec::Fragment(Vec::new());
    }
    let DynamicSpec::Fragment(children) = dynamic else {
        unreachable!();
    };
    Some(children)
}

fn selected_existing_fragment_mut(
    vnode: &mut VNodeSpec,
    selector: u8,
) -> Option<&mut Vec<VNodeSpec>> {
    match selected_dynamic_mut(vnode, selector)? {
        DynamicSpec::Fragment(children) => Some(children),
        _ => None,
    }
}

fn apply_fragment_key_mode(children: &mut [VNodeSpec], mode: &FragmentKeyMode) {
    for (index, child) in children.iter_mut().enumerate() {
        child.key = match mode {
            FragmentKeyMode::Unkeyed => None,
            FragmentKeyMode::Keyed { base } => Some(base.wrapping_add(index as u8)),
        };
    }
}

fn insert_fragment_child(children: &mut Vec<VNodeSpec>, index: u8, key: Option<u8>) {
    if children.len() >= MAX_FRAGMENT_CHILDREN {
        return;
    }
    let mut child = VNodeSpec::minimal();
    child.key = fragment_child_key(children, key);
    let index = insert_index(children.len(), index);
    children.insert(index, child);
}

fn fragment_child_key(children: &[VNodeSpec], requested: Option<u8>) -> Option<u8> {
    match children.first().and_then(|child| child.key) {
        Some(_) => Some(unique_fragment_key(children, requested.unwrap_or(0))),
        None if children.is_empty() => requested,
        None => None,
    }
}

fn unique_fragment_key(children: &[VNodeSpec], mut candidate: u8) -> u8 {
    while children.iter().any(|child| child.key == Some(candidate)) {
        candidate = candidate.wrapping_add(1);
    }
    candidate
}
