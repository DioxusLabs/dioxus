use crate::{context::HarnessContext, model::*};
use mutatis::{Candidates, DefaultMutate, Generate, Mutate, Result as MutatisResult};
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll, Waker},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum Op {
    Rerender,
    WakeSuspense {
        suspense: u8,
    },
    FireEvent {
        target: u8,
        behavior: EventBehaviorSpec,
    },
    Mutate(ModelEdit),
}

impl Op {
    pub(crate) fn wake_suspense(suspense: u8) -> Self {
        Self::WakeSuspense { suspense }
    }

    pub(crate) fn fire_event(target: u8, behavior: EventBehaviorSpec) -> Self {
        Self::FireEvent { target, behavior }
    }

    pub(crate) fn template(vnode: u8, edit: TemplateEdit) -> Self {
        Self::Mutate(ModelEdit::VNode { vnode, edit })
    }

    pub(crate) fn dynamic(vnode: u8, node: u8, kind: DynamicKind) -> Self {
        Self::Mutate(ModelEdit::VNode {
            vnode,
            edit: TemplateEdit::SetNode {
                node,
                kind: TemplateNodeKind::Dynamic(kind),
            },
        })
    }

    pub(crate) fn dynamic_attrs(vnode: u8, attr: u8, edit: ListEdit<AttrSpec>) -> Self {
        Self::Mutate(ModelEdit::VNode {
            vnode,
            edit: TemplateEdit::DynamicAttrs { attr, edit },
        })
    }

    pub(crate) fn fragment(vnode: u8, node: u8, edit: FragmentEdit) -> Self {
        Self::Mutate(ModelEdit::VNode {
            vnode,
            edit: TemplateEdit::Fragment { node, edit },
        })
    }

    pub(crate) fn suspense(suspense: u8, mode: SuspenseMode) -> Self {
        Self::Mutate(ModelEdit::Suspense {
            suspense,
            edit: SuspenseEdit::Mode(mode),
        })
    }

    pub(crate) fn suspense_wake_mutation(suspense: u8, mutation: WakeMutationSpec) -> Self {
        Self::Mutate(ModelEdit::Suspense {
            suspense,
            edit: SuspenseEdit::WakeMutation(mutation),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum EventBehaviorSpec {
    Noop,
    DispatchNestedEvent { target: u8 },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum ModelEdit {
    VNode { vnode: u8, edit: TemplateEdit },
    Suspense { suspense: u8, edit: SuspenseEdit },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum SuspenseEdit {
    Mode(SuspenseMode),
    WakeMutation(WakeMutationSpec),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
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
    Fragment {
        node: u8,
        edit: FragmentEdit,
    },
    DynamicAttrs {
        attr: u8,
        edit: ListEdit<AttrSpec>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum FragmentEdit {
    KeyMode(FragmentKeyMode),
    Children(ListEdit<Option<u8>>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Default)]
pub(crate) struct SuspenseReadyRegistry {
    wake_counts: Vec<(SuspenseReadyKey, usize)>,
    wakers: Vec<(SuspenseReadyKey, Waker)>,
}

impl SuspenseReadyRegistry {
    fn wake_count(&self, key: SuspenseReadyKey) -> usize {
        self.wake_counts
            .iter()
            .find_map(|(wake_key, count)| (*wake_key == key).then_some(*count))
            .unwrap_or(0)
    }

    fn released(&self, key: SuspenseReadyKey, required_wakes: usize) -> bool {
        self.wake_count(key) >= required_wakes
    }

    fn register_waker(&mut self, key: SuspenseReadyKey, waker: Waker) {
        if let Some((_, existing)) = self
            .wakers
            .iter_mut()
            .find(|(wake_key, _)| *wake_key == key)
        {
            *existing = waker;
        } else {
            self.wakers.push((key, waker));
        }
    }

    fn release(&mut self, key: SuspenseReadyKey) {
        if let Some((_, count)) = self
            .wake_counts
            .iter_mut()
            .find(|(wake_key, _)| *wake_key == key)
        {
            *count = count.saturating_add(1);
        } else {
            self.wake_counts.push((key, 1));
        }

        if let Some((_, waker)) = self.wakers.iter().find(|(wake_key, _)| *wake_key == key) {
            waker.wake_by_ref();
        }
    }

    fn registered_keys(&self) -> Vec<SuspenseReadyKey> {
        self.wakers.iter().map(|(key, _)| *key).collect()
    }

    fn clear(&mut self) {
        self.wake_counts.clear();
        self.wakers.clear();
    }
}

struct SuspenseReadyRegistrationGuard {
    context: HarnessContext,
    previous: bool,
}

impl Drop for SuspenseReadyRegistrationGuard {
    fn drop(&mut self) {
        self.context
            .register_suspense_ready_wakers
            .set(self.previous);
    }
}

impl HarnessContext {
    pub(crate) fn read_model(&self) -> Model {
        self.model.borrow().clone()
    }

    pub(crate) fn with_model<R>(&self, f: impl FnOnce(&mut Model) -> R) -> R {
        f(&mut self.model.borrow_mut())
    }

    fn suspense_ready_released(&self, key: SuspenseReadyKey, required_wakes: usize) -> bool {
        self.register_suspense_ready_wakers.get()
            && self.suspense_ready.borrow().released(key, required_wakes)
    }

    fn register_suspense_ready_waker(&self, key: SuspenseReadyKey, waker: Waker) {
        if self.register_suspense_ready_wakers.get() {
            self.suspense_ready.borrow_mut().register_waker(key, waker);
        }
    }

    pub(crate) fn release_suspense_ready_task(&self, key: SuspenseReadyKey) {
        self.suspense_ready.borrow_mut().release(key);
    }

    pub(crate) fn selected_registered_ready_suspense_key(
        &self,
        selector: u8,
    ) -> Option<SuspenseReadyKey> {
        let registered = self.suspense_ready.borrow().registered_keys();

        let mut ready = Vec::new();
        self.read_model()
            .root
            .collect_ready_suspense_keys(&mut ready);
        ready.retain(|key| registered.contains(key));
        select(ready, selector)
    }

    pub(crate) fn clear_suspense_ready_tasks(&self) {
        self.suspense_ready.borrow_mut().clear();
    }

    pub(crate) fn without_suspense_ready_registration<R>(&self, f: impl FnOnce() -> R) -> R {
        let previous = self.register_suspense_ready_wakers.replace(false);
        let _guard = SuspenseReadyRegistrationGuard {
            context: self.clone(),
            previous,
        };
        f()
    }

    pub(crate) fn apply_to_model(&self, op: &Op) {
        let Op::Mutate(edit) = op else {
            return;
        };

        self.with_model(|model| {
            let can_grow = model.can_grow();
            apply_model_edit(model, edit, can_grow);
        });
    }
}

pub(crate) struct SuspenseReadyFuture {
    pub(crate) context: HarnessContext,
    pub(crate) key: SuspenseReadyKey,
    pub(crate) required_wakes: usize,
}

impl Future for SuspenseReadyFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let key = self.key;
        if self
            .context
            .suspense_ready_released(key, self.required_wakes)
        {
            Poll::Ready(())
        } else {
            self.context
                .register_suspense_ready_waker(key, cx.waker().clone());
            Poll::Pending
        }
    }
}

pub(crate) fn apply_strategy_op_to_model(model: &mut Model, op: &Op) {
    if matches!(op, Op::Rerender | Op::FireEvent { .. }) {
        return;
    }

    let can_grow = model.can_grow();
    match op {
        Op::Rerender => {}
        Op::FireEvent { .. } => {}
        Op::WakeSuspense { suspense } => {
            if let Some(key) = model.selected_ready_suspense_key(*suspense) {
                model.wake_ready_suspense(key);
            }
        }
        Op::Mutate(edit) => apply_model_edit(model, edit, can_grow),
    }
}

fn apply_model_edit(model: &mut Model, edit: &ModelEdit, can_grow: bool) {
    match edit {
        ModelEdit::VNode { vnode, edit } => apply_vnode_edit(model, *vnode, edit, can_grow),
        ModelEdit::Suspense { suspense, edit } => match edit {
            SuspenseEdit::Mode(mode) => model.set_selected_suspense_mode(*suspense, *mode),
            SuspenseEdit::WakeMutation(mutation) => {
                model.set_selected_suspense_wake_mutation(*suspense, *mutation);
            }
        },
    }
}

fn apply_vnode_edit(model: &mut Model, vnode: u8, edit: &TemplateEdit, can_grow: bool) {
    let mut next_suspense_id = model.next_suspense_id;
    let mut next_component_id = model.next_component_id;
    {
        let vnode = model.selected_vnode_mut(vnode);
        apply_template_edit(
            vnode,
            edit,
            can_grow,
            &mut next_suspense_id,
            &mut next_component_id,
        );
        vnode.normalize_in_place();
    }
    model.next_suspense_id = next_suspense_id;
    model.next_component_id = next_component_id;
}

fn apply_template_edit(
    vnode: &mut VNodeSpec,
    edit: &TemplateEdit,
    can_grow: bool,
    next_suspense_id: &mut u64,
    next_component_id: &mut u64,
) {
    match edit {
        TemplateEdit::SetNode { node, kind } => {
            vnode.template.cache_key = None;
            if let Some(path) = select(vnode.template.node_paths(), *node) {
                if let Some(node) = vnode.template.node_mut(&path) {
                    if can_apply_template_node_kind(kind, can_grow) {
                        node.set_kind(kind, next_suspense_id, next_component_id);
                    }
                }
            }
        }
        TemplateEdit::Roots { edit } => {
            vnode.template.cache_key = None;
            apply_template_node_list_edit(
                &mut vnode.template.roots,
                edit,
                1,
                MAX_ROOTS,
                can_grow,
                next_suspense_id,
                next_component_id,
            );
        }
        TemplateEdit::Children { element, edit } => {
            vnode.template.cache_key = None;
            if let Some(path) = select(vnode.template.element_paths(), *element) {
                if let Some(TemplateNodeSpec::Element { children, .. }) =
                    vnode.template.element_mut(&path)
                {
                    apply_template_node_list_edit(
                        children,
                        edit,
                        0,
                        MAX_CHILDREN,
                        can_grow,
                        next_suspense_id,
                        next_component_id,
                    );
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
        TemplateEdit::Fragment { node, edit } => {
            apply_fragment_edit(vnode, *node, edit, can_grow);
        }
        TemplateEdit::DynamicAttrs { attr, edit } => {
            if let Some(attrs) = selected_dynamic_attr_mut(vnode, *attr) {
                apply_attr_list_edit(attrs, edit);
            }
        }
    }
}

fn can_apply_template_node_kind(kind: &TemplateNodeKind, can_grow: bool) -> bool {
    can_grow
        || matches!(
            kind,
            TemplateNodeKind::Element { .. }
                | TemplateNodeKind::Text(_)
                | TemplateNodeKind::Dynamic(
                    DynamicKind::Empty | DynamicKind::Text(_) | DynamicKind::Placeholder
                )
                | TemplateNodeKind::Dynamic(DynamicKind::Fragment { children: 0, .. })
        )
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
    next_suspense_id: &mut u64,
    next_component_id: &mut u64,
) {
    match edit {
        ListEdit::Insert { index, item } => {
            if can_grow && nodes.len() < max_len {
                let index = insert_index(nodes.len(), *index);
                nodes.insert(
                    index,
                    TemplateNodeSpec::from_kind(item, next_suspense_id, next_component_id),
                );
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
    let dynamic_count = vnode.template.dynamic_count();
    if dynamic_count == 0 {
        return None;
    }
    let mut index = selector as usize % dynamic_count;
    vnode.template.nth_dynamic_mut(&mut index)
}

fn selected_dynamic_attr_mut(vnode: &mut VNodeSpec, selector: u8) -> Option<&mut Vec<AttrSpec>> {
    let attr_count = vnode.template.attr_count();
    if attr_count == 0 {
        return None;
    }
    let mut index = selector as usize % attr_count;
    vnode.template.nth_dynamic_attr_mut(&mut index)
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
