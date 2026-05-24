use crate::{
    DynamicNode, ElementId, RenderTargetId, ScopeId, VComponent, VirtualDom,
    diff::{
        anchor::{Anchor, anchor_after, anchor_before, at_anchor, create_at_anchor},
        context::{DiffContext, DiffFrame, DiffState},
    },
    innerlude::{ComponentPropsUpdate, ElementRef, MountId, WriteMutations},
    nodes::VNode,
};

use rustc_hash::{FxHashMap, FxHashSet};

type AnchorFn = for<'a> fn(&VNode, &[MountId], &VirtualDom, Option<DiffContext<'a>>) -> Anchor;
const FRAGMENT_WORK_BATCH: usize = 16;

impl<M: WriteMutations> DiffState<'_, M> {
    pub(crate) fn diff_non_empty_fragment(
        &mut self,
        old: &[VNode],
        new: &[VNode],
        parent: Option<ElementRef>,
    ) {
        let new_is_keyed = new[0].key.is_some();
        let old_is_keyed = old[0].key.is_some();
        debug_assert!(
            new.iter().all(|n| n.key.is_some() == new_is_keyed),
            "all siblings must be keyed or all siblings must be non-keyed"
        );
        debug_assert!(
            old.iter().all(|o| o.key.is_some() == old_is_keyed),
            "all siblings must be keyed or all siblings must be non-keyed"
        );

        if new_is_keyed && old_is_keyed {
            self.diff_keyed_children(old, new, parent);
        } else {
            self.diff_non_keyed_children(old, new, parent);
        }
    }

    fn diff_non_keyed_children(
        &mut self,
        old: &[VNode],
        new: &[VNode],
        parent: Option<ElementRef>,
    ) {
        use std::cmp::Ordering;

        debug_assert!(!new.is_empty());
        debug_assert!(!old.is_empty());

        match old.len().cmp(&new.len()) {
            Ordering::Greater => self
                .dom
                .remove_nodes(self.to.as_deref_mut(), &old[new.len()..]),
            Ordering::Less => {
                self.create_and_insert(anchor_after, &new[old.len()..], old.last().unwrap(), parent)
            }
            Ordering::Equal => {}
        }

        self.diff_child_pairs(old.iter(), new);
    }

    fn diff_keyed_children(&mut self, old: &[VNode], new: &[VNode], parent: Option<ElementRef>) {
        let Some((left_offset, right_offset)) = self.diff_keyed_ends(old, new, parent) else {
            return;
        };

        let old_middle = &old[left_offset..(old.len() - right_offset)];
        let new_middle = &new[left_offset..(new.len() - right_offset)];

        if !old_middle.is_empty()
            && !new_middle.is_empty()
            && !has_shared_key(old_middle, new_middle)
            && (left_offset > 0 || right_offset > 0)
        {
            if right_offset > 0 {
                // The right-edge pairs were already diffed by
                // `diff_keyed_ends`, so the matching new vnode has its mount.
                self.create_and_insert(
                    anchor_before,
                    new_middle,
                    &new[new.len() - right_offset],
                    parent,
                );
            } else {
                // The left-edge pairs are diffed *after* this splice by
                // `diff_shared_prefix`, so the matching new vnode's mount
                // cell is still unset. Use the OLD sibling instead — its
                // mount still references the element we want to anchor next
                // to. (Anchoring against the unmounted new sibling falls
                // through to `Anchor::AppendTo(ROOT)` and lands the new
                // content past unrelated root siblings.)
                self.create_and_insert(anchor_after, new_middle, &old[left_offset - 1], parent);
            }
            self.dom.remove_nodes(self.to.as_deref_mut(), old_middle);
        } else {
            self.diff_keyed_middle(old_middle, new_middle, parent);
        }
        self.diff_shared_prefix(old, new, left_offset);
    }

    fn diff_keyed_ends(
        &mut self,
        old: &[VNode],
        new: &[VNode],
        parent: Option<ElementRef>,
    ) -> Option<(usize, usize)> {
        let left_offset = old
            .iter()
            .zip(new.iter())
            .take_while(|(o, n)| o.key == n.key)
            .count();
        let right_offset = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take_while(|(o, n)| o.key == n.key)
            .take(old.len().min(new.len()) - left_offset)
            .count();

        for (old, new) in old.iter().rev().zip(new.iter().rev()).take(right_offset) {
            DiffFrame::new(old.mount.get(), old, new).diff_into(self);
        }

        let retained = right_offset + left_offset;
        if left_offset == old.len()
            || right_offset == old.len()
            || retained == new.len()
            || retained == old.len()
        {
            self.diff_shared_prefix(old, new, left_offset);
            if left_offset == old.len() {
                self.create_and_insert(
                    anchor_after,
                    &new[left_offset..],
                    &new[left_offset - 1],
                    parent,
                );
            } else if right_offset == old.len() {
                self.create_and_insert(
                    anchor_before,
                    &new[..new.len() - right_offset],
                    &new[new.len() - right_offset],
                    parent,
                );
            } else if retained == new.len() {
                self.dom.remove_nodes(
                    self.to.as_deref_mut(),
                    &old[left_offset..old.len() - right_offset],
                );
            } else {
                self.create_and_insert(
                    anchor_before,
                    &new[left_offset..new.len() - right_offset],
                    &new[new.len() - right_offset],
                    parent,
                );
            }
            return None;
        }

        Some((left_offset, right_offset))
    }

    fn diff_shared_prefix(&mut self, old: &[VNode], new: &[VNode], len: usize) {
        self.diff_child_pairs(old.iter().take(len), &new[..len]);
    }

    fn diff_child_pairs<'a>(&mut self, old: impl Iterator<Item = &'a VNode>, new: &'a [VNode]) {
        let pairs = old.zip(new.iter()).collect::<Vec<_>>();
        if new.len() > FRAGMENT_WORK_BATCH {
            let mut updates = Vec::with_capacity(pairs.len());
            for (old, new) in &pairs {
                let Some(update) = self.component_props_update(old, new) else {
                    for (old, new) in pairs.into_iter().rev() {
                        DiffFrame::new(old.mount.get(), old, new).diff_into(self);
                    }
                    return;
                };
                updates.push(update);
            }

            for batch in updates.chunks(FRAGMENT_WORK_BATCH) {
                self.dom
                    .queue_component_props_diff(self.priority, batch.to_vec());
            }
        } else {
            for (old, new) in pairs.into_iter().rev() {
                DiffFrame::new(old.mount.get(), old, new).diff_into(self);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn diff_keyed_middle(&mut self, old: &[VNode], new: &[VNode], parent: Option<ElementRef>) {
        debug_assert_ne!(new.first().map(|i| &i.key), old.first().map(|i| &i.key));
        debug_assert_ne!(new.last().map(|i| &i.key), old.last().map(|i| &i.key));

        let old_key_to_old_index = old
            .iter()
            .enumerate()
            .map(|(i, o)| (o.key.as_ref().unwrap().as_str(), i))
            .collect::<FxHashMap<_, _>>();

        let mut shared_keys = FxHashSet::default();

        let new_index_to_old_index = new
            .iter()
            .map(|node| {
                let key = node.key.as_ref().unwrap();
                if let Some(&index) = old_key_to_old_index.get(key.as_str()) {
                    shared_keys.insert(key);
                    index
                } else {
                    usize::MAX
                }
            })
            .collect::<Box<[_]>>();

        if shared_keys.is_empty() {
            debug_assert!(
                !old.is_empty(),
                "we should never be appending - just creating N"
            );
            let first_old = old.first().unwrap();
            let anchor = anchor_before(first_old, &[], self.dom, self.context());
            create_at_anchor(new, parent, anchor, self.dom, self.to.as_deref_mut());
            self.dom.remove_nodes(self.to.as_deref_mut(), old);
            return;
        }

        for child_to_remove in old
            .iter()
            .filter(|child| !shared_keys.contains(child.key.as_ref().unwrap()))
        {
            child_to_remove.remove_node(self.dom, self.to.as_deref_mut());
        }

        let mut lis_sequence = Vec::with_capacity(new_index_to_old_index.len());

        let mut allocation = vec![0; new_index_to_old_index.len() * 2];
        let (predecessors, starts) = allocation.split_at_mut(new_index_to_old_index.len());

        longest_increasing_subsequence::lis_with(
            &new_index_to_old_index,
            &mut lis_sequence,
            |a, b| a < b,
            predecessors,
            starts,
        );

        if lis_sequence.first().map(|f| new_index_to_old_index[*f]) == Some(usize::MAX) {
            lis_sequence.remove(0);
        }

        for idx in &lis_sequence {
            let old_node = &old[new_index_to_old_index[*idx]];
            DiffFrame::new(old_node.mount.get(), old_node, &new[*idx]).diff_into(self);
        }

        let last = *lis_sequence.first().unwrap();
        if last < (new.len() - 1) {
            self.splice_around_diffing(
                anchor_after,
                new,
                old,
                &new[last],
                parent,
                &new_index_to_old_index,
                (last + 1)..new.len(),
            );
        }

        for pair in lis_sequence.windows(2) {
            let (last, next) = (pair[0], pair[1]);
            if last - next > 1 {
                self.splice_around_diffing(
                    anchor_before,
                    new,
                    old,
                    &new[last],
                    parent,
                    &new_index_to_old_index,
                    (next + 1)..last,
                );
            }
        }

        let first_lis = *lis_sequence.last().unwrap();
        if first_lis > 0 {
            self.splice_around_diffing(
                anchor_before,
                new,
                old,
                &new[first_lis],
                parent,
                &new_index_to_old_index,
                0..first_lis,
            );
        }
    }

    fn splice_around_diffing(
        &mut self,
        anchor: AnchorFn,
        new: &[VNode],
        old: &[VNode],
        sibling: &VNode,
        parent: Option<ElementRef>,
        new_index_to_old_index: &[usize],
        range: std::ops::Range<usize>,
    ) {
        let skip = collect_splice_mounts(old, new_index_to_old_index, range.clone());
        let context = self.context();
        let anchor = anchor(sibling, &skip, self.dom, context);
        let dom = &mut *self.dom;
        let to = self.to.as_deref_mut();
        at_anchor(anchor, to, |to| {
            let mut state = DiffState::new_with_context(dom, to, context);
            state.create_or_diff_range(new, old, parent, new_index_to_old_index, range)
        });
    }

    fn create_or_diff_range(
        &mut self,
        new: &[VNode],
        old: &[VNode],
        parent: Option<ElementRef>,
        new_index_to_old_index: &[usize],
        range: std::ops::Range<usize>,
    ) -> usize {
        let range_start = range.start;
        let mut nodes = 0;
        for (idx, new_node) in new[range].iter().enumerate() {
            let old_index = new_index_to_old_index[range_start + idx];
            nodes += if let Some(old_node) = old.get(old_index) {
                DiffFrame::new(old_node.mount.get(), old_node, new_node).diff_into(self);
                self.to
                    .as_deref_mut()
                    .map_or(0, |to| new_node.push_all_root_nodes(self.dom, to))
            } else {
                new_node.create(self.dom, parent, self.to.as_deref_mut())
            };
        }
        nodes
    }

    fn create_and_insert(
        &mut self,
        anchor: AnchorFn,
        new: &[VNode],
        sibling: &VNode,
        parent: Option<ElementRef>,
    ) {
        let anchor = anchor(sibling, &collect_mounts(new), self.dom, self.context());
        create_at_anchor(new, parent, anchor, self.dom, self.to.as_deref_mut());
    }

    fn component_props_update(&self, old: &VNode, new: &VNode) -> Option<ComponentPropsUpdate> {
        if old.template != new.template {
            return None;
        }

        let (old_idx, old_component) = single_root_component(old)?;
        let (new_idx, new_component) = single_root_component(new)?;
        if old_idx != new_idx || old_component.render_fn != new_component.render_fn {
            return None;
        }

        // `old` came straight from the previous render's `dynamic_nodes` —
        // the diff would have skipped this fast path if `old` were a hole or
        // a never-mounted placeholder (`single_root_component` already
        // requires a single Component dynamic root). So `old.mount` is live
        // here by construction.
        let mount = old.mount.get();
        debug_assert!(mount.mounted(), "batched component_props_update requires mounted old");
        new.mount.set(mount);
        Some(ComponentPropsUpdate {
            scope: ScopeId(self.dom.get_mounted_dyn_node(mount, old_idx)),
            props: new_component.props.duplicate(),
        })
    }
}

fn single_root_component(vnode: &VNode) -> Option<(usize, &VComponent)> {
    if vnode.template.roots().len() != 1 {
        return None;
    }
    let (idx, node) = vnode.get_dynamic_root_node_and_id(0)?;
    match node {
        DynamicNode::Component(component) => Some((idx, component)),
        _ => None,
    }
}

fn has_shared_key(old: &[VNode], new: &[VNode]) -> bool {
    let old_keys = old
        .iter()
        .map(|child| child.key.as_ref().unwrap().as_str())
        .collect::<FxHashSet<_>>();

    new.iter()
        .any(|child| old_keys.contains(child.key.as_ref().unwrap().as_str()))
}

fn collect_mounts(nodes: &[VNode]) -> Vec<MountId> {
    nodes
        .iter()
        .map(|v| v.mount.get())
        .filter(|m| m.mounted())
        .collect()
}

fn collect_splice_mounts(
    old: &[VNode],
    new_index_to_old_index: &[usize],
    range: std::ops::Range<usize>,
) -> Vec<MountId> {
    // Each splice range is the *non-LIS* portion of the keyed middle, so the
    // new sibling at `range` is not yet claimed; only the matching old vnode's
    // live mount needs to be added to the skip list so anchor lookups don't
    // try to use a sibling that's about to be moved. The non-LIS old entries
    // come straight from the previous render with their mounts intact — no
    // earlier diff step has called `claim_fiber_mount` on them yet — so the
    // mount is always live by the time we collect it.
    range
        .filter_map(|idx| old.get(new_index_to_old_index[idx]))
        .map(|old_node| {
            let old_mount = old_node.mount.get();
            debug_assert!(old_mount.mounted(), "non-LIS splice old mount must be live");
            old_mount
        })
        .collect()
}

impl VNode {
    /// Push all the root nodes on the stack
    pub(crate) fn push_all_root_nodes(
        &self,
        dom: &VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        let mount = self.mount.get();
        let target_id = dom.current_render_target_id();

        self.template
            .roots()
            .iter()
            .enumerate()
            .map(
                |(root_idx, _)| match self.get_dynamic_root_node_and_id(root_idx) {
                    Some((_, DynamicNode::Fragment(nodes))) => nodes
                        .iter()
                        .map(|node| node.push_all_root_nodes(dom, to))
                        .sum(),
                    Some((idx, DynamicNode::Component(_))) => dom
                        .get_scope(ScopeId(dom.get_mounted_dyn_node(mount, idx)))
                        .unwrap()
                        .root_node()
                        .push_all_root_nodes(dom, to),
                    // For a single dynamic node of Text, push its element id
                    Some((idx, DynamicNode::Text(_))) => {
                        if dom.mount_target_id(mount) == target_id {
                            let id = ElementId(dom.get_mounted_dyn_node(mount, idx));
                            push_live_root(dom, to, target_id, id)
                        } else {
                            0
                        }
                    }
                    // This is a static root node or a single dynamic node, just push it
                    None => {
                        if dom.mount_target_id(mount) == target_id {
                            let id = dom.get_mounted_root_node(mount, root_idx);
                            push_live_root(dom, to, target_id, id)
                        } else {
                            0
                        }
                    }
                },
            )
            .sum()
    }
}

fn push_live_root(
    dom: &VirtualDom,
    to: &mut impl WriteMutations,
    target_id: RenderTargetId,
    id: ElementId,
) -> usize {
    // Callers (`push_all_root_nodes`) only reach this with `id` values just
    // read from `get_mounted_root_node`/`get_mounted_dyn_node` for a vnode
    // whose mount target already matches `target_id`, so the live element id
    // has been allocated in that target by `load_template_root` /
    // `assign_node_id`. The defensive id-validity check here is therefore
    // dead in any reachable diff path.
    debug_assert!(
        id.0 != 0
            && id.0 != usize::MAX
            && dom.element_exists_in_target(target_id, id),
        "push_live_root requires a live element id in the current target"
    );
    to.push_root(id);
    1
}
