use crate::{
    DynamicNode, ElementId, VirtualDom,
    diff::{
        anchor::{ElementEdge, anchor_at, at_anchor, create_at_anchor},
        context::{DiffFrame, DiffState},
    },
    innerlude::{ElementRef, MountId, WriteMutations},
    mutations::reborrow_writer,
    nodes::VNode,
};

use super::template::{TemplateRoot, template_roots};

use rustc_hash::{FxHashMap, FxHashSet};

impl DiffState<'_, '_, '_> {
    pub(crate) fn diff_non_empty_fragment(
        &mut self,
        old: &[VNode],
        new: &[VNode],
        parent: Option<ElementRef>,
    ) {
        let new_is_keyed = new[0].key.is_some();
        let old_is_keyed = old[0].key.is_some();
        dioxus_debug_assert!(
            new.iter().all(|n| n.key.is_some() == new_is_keyed),
            "all siblings must be keyed or all siblings must be non-keyed"
        );
        dioxus_debug_assert!(
            old.iter().all(|o| o.key.is_some() == old_is_keyed),
            "all siblings must be keyed or all siblings must be non-keyed"
        );

        if new_is_keyed && old_is_keyed {
            self.diff_keyed_children(old, new, parent);
        } else {
            self.diff_non_keyed_children(old, new, parent);
        }
    }

    // Diff children that are not keyed.
    //
    // The parent must be on the top of the change list stack when entering this
    // function:
    //
    //     [... parent]
    //
    // the change list stack is in the same state when this function returns.
    fn diff_non_keyed_children(
        &mut self,
        old: &[VNode],
        new: &[VNode],
        parent: Option<ElementRef>,
    ) {
        use std::cmp::Ordering;

        // Handled these cases in `diff_children` before calling this function.
        dioxus_debug_assert!(!new.is_empty());
        dioxus_debug_assert!(!old.is_empty());

        match old.len().cmp(&new.len()) {
            Ordering::Greater => self
                .dom
                .remove_nodes(reborrow_writer(&mut self.to), &old[new.len()..]),
            Ordering::Less => self.create_and_insert(
                ElementEdge::Last,
                &new[old.len()..],
                old.last().unwrap(),
                parent,
            ),
            Ordering::Equal => {}
        }

        self.diff_child_pairs(old, new);
    }

    // Diffing "keyed" children.
    //
    // With keyed children, we care about whether we delete, move, or create nodes
    // versus mutate existing nodes in place. Presumably there is some sort of CSS
    // transition animation that makes the virtual DOM diffing algorithm
    // observable. By specifying keys for nodes, we know which virtual DOM nodes
    // must reuse (or not reuse) the same physical DOM nodes.
    //
    // This is loosely based on Inferno's keyed patching implementation. However, we
    // have to modify the algorithm since we are compiling the diff down into change
    // list instructions that will be executed later, rather than applying the
    // changes to the DOM directly as we compare virtual DOMs.
    //
    // https://github.com/infernojs/inferno/blob/36fd96/packages/inferno/src/DOM/patching.ts#L530-L739
    //
    // The stack is empty upon entry.
    fn diff_keyed_children(&mut self, old: &[VNode], new: &[VNode], parent: Option<ElementRef>) {
        #[cfg(debug_assertions)]
        {
            let mut keys = rustc_hash::FxHashSet::default();
            let mut assert_unique_keys = |children: &[VNode]| {
                keys.clear();
                for child in children {
                    let key = child.key.clone();
                    dioxus_debug_assert!(
                        key.is_some(),
                        "if any sibling is keyed, all siblings must be keyed"
                    );
                    keys.insert(key);
                }
                dioxus_debug_assert_eq!(
                    children.len(),
                    keys.len(),
                    "keyed siblings must each have a unique key"
                );
            };
            assert_unique_keys(old);
            assert_unique_keys(new);
        }

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
                    ElementEdge::First,
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
                self.create_and_insert(
                    ElementEdge::Last,
                    new_middle,
                    &old[left_offset - 1],
                    parent,
                );
            }
            self.dom
                .remove_nodes(reborrow_writer(&mut self.to), old_middle);
        } else {
            self.diff_keyed_middle(old_middle, new_middle, parent);
        }
        self.diff_shared_prefix(old, new, left_offset);
    }

    /// Diff both ends of the children that share keys.
    ///
    /// Returns a left offset and right offset of that indicates a smaller section to pass onto the middle diffing.
    ///
    /// If there is no offset, then this function returns None and the diffing is complete.
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
            DiffFrame::new(old.unchecked_mounted_id(), old, new).diff_into(self);
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
                    ElementEdge::Last,
                    &new[left_offset..],
                    &new[left_offset - 1],
                    parent,
                );
            } else if right_offset == old.len() {
                self.create_and_insert(
                    ElementEdge::First,
                    &new[..new.len() - right_offset],
                    &new[new.len() - right_offset],
                    parent,
                );
            } else if retained == new.len() {
                self.dom.remove_nodes(
                    reborrow_writer(&mut self.to),
                    &old[left_offset..old.len() - right_offset],
                );
            } else {
                self.create_and_insert(
                    ElementEdge::First,
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
        self.diff_child_pairs(&old[..len], &new[..len]);
    }

    fn diff_child_pairs(&mut self, old: &[VNode], new: &[VNode]) {
        let len = old.len().min(new.len());
        for idx in (0..len).rev() {
            let old = &old[idx];
            let new = &new[idx];
            DiffFrame::new(old.unchecked_mounted_id(), old, new).diff_into(self);
        }
    }

    // The most-general, expensive code path for keyed children diffing.
    //
    // We find the longest subsequence within `old` of children that are relatively
    // ordered the same way in `new` (via finding a longest-increasing-subsequence
    // of the old child's index within `new`). The children that are elements of
    // this subsequence will remain in place, minimizing the number of DOM moves we
    // will have to do.
    #[allow(clippy::too_many_lines)]
    fn diff_keyed_middle(&mut self, old: &[VNode], new: &[VNode], parent: Option<ElementRef>) {
        /*
        1. Map the old keys into a numerical ordering based on indices.
        2. Create a map of old key to its index
        3. Map each new key to the old key, carrying over the old index.
            - IE if we have ABCD becomes BACD, our sequence would be 1,0,2,3
            - if we have ABCD to ABDE, our sequence would be 0,1,3,MAX because E doesn't exist

        now, we should have a list of integers that indicates where in the old list the new items map to.

        4. Compute the LIS of this list
            - this indicates the longest list of new children that won't need to be moved.

        5. Identify which nodes need to be removed
        6. Identify which nodes will need to be diffed

        7. Going along each item in the new list, create it and insert it before the next closest item in the LIS.
            - if the item already existed, just move it to the right place.

        8. Finally, generate instructions to remove any old children.
        9. Generate instructions to finally diff children that are the same between both
        */
        // 0. Debug sanity checks
        // Should have already diffed the shared-key prefixes and suffixes.
        dioxus_debug_assert_ne!(new.first().map(|i| &i.key), old.first().map(|i| &i.key));
        dioxus_debug_assert_ne!(new.last().map(|i| &i.key), old.last().map(|i| &i.key));

        // 1. Map the old keys into a numerical ordering based on indices.
        // 2. Create a map of old key to its index
        // IE if the keys were A B C, then we would have (A, 0) (B, 1) (C, 2).
        let old_key_to_old_index = old
            .iter()
            .enumerate()
            .map(|(i, o)| (o.key.as_ref().unwrap().as_str(), i))
            .collect::<FxHashMap<_, _>>();

        let mut shared_keys = FxHashSet::default();

        // 3. Map each new key to the old key, carrying over the old index.
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

        // If none of the old keys are reused by the new children, then we remove all the remaining old children and
        // create the new children afresh.
        if shared_keys.is_empty() {
            let first_old = old.first().unwrap();
            let anchor = anchor_at(ElementEdge::First, first_old, &[], self.dom, self.context());
            create_at_anchor(new, parent, anchor, self.dom, reborrow_writer(&mut self.to));
            self.dom.remove_nodes(reborrow_writer(&mut self.to), old);
            return;
        }

        // remove any old children that are not shared
        for child_to_remove in old
            .iter()
            .filter(|child| !shared_keys.contains(child.key.as_ref().unwrap()))
        {
            child_to_remove.remove_node(self.dom, reborrow_writer(&mut self.to));
        }

        // 4. Compute the LIS of this list
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

        // if a new node gets u32 max and is at the end, then it might be part of our LIS (because u32 max is a valid LIS)
        if lis_sequence.first().map(|f| new_index_to_old_index[*f]) == Some(usize::MAX) {
            lis_sequence.remove(0);
        }

        // Diff each nod in the LIS
        for idx in &lis_sequence {
            let old_node = &old[new_index_to_old_index[*idx]];
            DiffFrame::new(old_node.unchecked_mounted_id(), old_node, &new[*idx]).diff_into(self);
        }

        // add mount instruction for the items before the LIS
        let last = *lis_sequence.first().unwrap();
        if last < (new.len() - 1) {
            self.splice_around_diffing(
                ElementEdge::Last,
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
                    ElementEdge::First,
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
                ElementEdge::First,
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
        edge: ElementEdge,
        new: &[VNode],
        old: &[VNode],
        sibling: &VNode,
        parent: Option<ElementRef>,
        new_index_to_old_index: &[usize],
        range: std::ops::Range<usize>,
    ) {
        let skip = collect_splice_mounts(old, new_index_to_old_index, range.clone());
        let context = self.context();
        let anchor = anchor_at(edge, sibling, &skip, self.dom, context);
        let runtime = self.dom.runtime.clone();
        let dom = &mut *self.dom;
        let to = reborrow_writer(&mut self.to);
        at_anchor(anchor, to, runtime, |to| {
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
                DiffFrame::new(old_node.unchecked_mounted_id(), old_node, new_node).diff_into(self);
                reborrow_writer(&mut self.to)
                    .map_or(0, |to| new_node.push_all_root_nodes(self.dom, to))
            } else {
                new_node.create(self.dom, parent, reborrow_writer(&mut self.to))
            };
        }
        nodes
    }

    fn create_and_insert(
        &mut self,
        edge: ElementEdge,
        new: &[VNode],
        sibling: &VNode,
        parent: Option<ElementRef>,
    ) {
        let anchor = anchor_at(
            edge,
            sibling,
            &collect_mounts(new),
            self.dom,
            self.context(),
        );
        create_at_anchor(new, parent, anchor, self.dom, reborrow_writer(&mut self.to));
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
    nodes.iter().filter_map(VNode::mounted_id).collect()
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
    // earlier diff step has called `claim_mount` on them yet — so the
    // mount is always live by the time we collect it.
    range
        .filter_map(|idx| old.get(new_index_to_old_index[idx]))
        .map(VNode::unchecked_mounted_id)
        .collect()
}

impl VNode {
    /// Push all the root nodes on the stack
    pub(crate) fn push_all_root_nodes(
        &self,
        dom: &VirtualDom,
        to: &mut dyn WriteMutations,
    ) -> usize {
        let mount = self.unchecked_mounted_id();
        let target_id = dom.current_render_target_id();

        let mut count = 0;
        for root in template_roots(self) {
            if let TemplateRoot::Static { root_idx, .. } = root {
                if dom.mount_target_id(mount) == target_id
                    && let Some(id) = dom.mounted_root_node(mount, root_idx)
                {
                    count += push_live_root(to, id.element_id());
                }
            } else if let TemplateRoot::Dynamic { slot } = root {
                count += self.push_dynamic_root_node(slot.index(), mount, target_id, dom, to);
            }
        }

        count
    }

    fn push_dynamic_root_node(
        &self,
        idx: usize,
        mount: MountId,
        target_id: crate::RenderTargetId,
        dom: &VirtualDom,
        to: &mut dyn WriteMutations,
    ) -> usize {
        match self.dynamic_values[idx].node() {
            DynamicNode::Fragment(nodes) => nodes
                .iter()
                .map(|node| node.push_all_root_nodes(dom, to))
                .sum(),
            DynamicNode::Component(_) => dom
                .get_scope(dom.unchecked_mounted_dynamic_component_scope(mount, idx))
                .unwrap()
                .root_node()
                .push_all_root_nodes(dom, to),
            DynamicNode::Text(_) => {
                if dom.mount_target_id(mount) == target_id {
                    let id = dom
                        .unchecked_mounted_dynamic_text_node(mount, idx)
                        .element_id();
                    push_live_root(to, id)
                } else {
                    0
                }
            }
        }
    }
}

fn push_live_root(to: &mut dyn WriteMutations, id: ElementId) -> usize {
    // Callers (`push_all_root_nodes`) only reach this with `id` values just
    // read from `unchecked_mounted_root_node`/`unchecked_mounted_dynamic_text_node` for a vnode
    // whose mount target already matches `target_id`, so the live element id
    // has been allocated in that target by `load_template_root` /
    // `assign_node_id`.
    to.push_id(id);
    1
}
