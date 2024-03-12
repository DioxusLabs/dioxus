use crate::{
    innerlude::{ElementRef, WriteMutations},
    nodes::VNode,
    DynamicNode, ScopeId, TemplateNode, VirtualDom,
};

use rustc_hash::{FxHashMap, FxHashSet};

impl VirtualDom {
    pub(crate) fn diff_non_empty_fragment(
        &mut self,
        to: &mut impl WriteMutations,
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
            self.diff_keyed_children(to, old, new, parent);
        } else {
            self.diff_non_keyed_children(to, old, new, parent);
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
        to: &mut impl WriteMutations,
        old: &[VNode],
        new: &[VNode],
        parent: Option<ElementRef>,
    ) {
        use std::cmp::Ordering;

        // Handled these cases in `diff_children` before calling this function.
        debug_assert!(!new.is_empty());
        debug_assert!(!old.is_empty());

        match old.len().cmp(&new.len()) {
            Ordering::Greater => self.remove_nodes(to, &old[new.len()..], None),
            Ordering::Less => {
                self.create_and_insert_after(to, &new[old.len()..], old.last().unwrap(), parent)
            }
            Ordering::Equal => {}
        }

        for (new, old) in new.iter().zip(old.iter()) {
            old.diff_node(new, self, to);
        }
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
    fn diff_keyed_children(
        &mut self,
        to: &mut impl WriteMutations,
        old: &[VNode],
        new: &[VNode],
        parent: Option<ElementRef>,
    ) {
        if cfg!(debug_assertions) {
            let mut keys = rustc_hash::FxHashSet::default();
            let mut assert_unique_keys = |children: &[VNode]| {
                keys.clear();
                for child in children {
                    let key = child.key.clone();
                    debug_assert!(
                        key.is_some(),
                        "if any sibling is keyed, all siblings must be keyed"
                    );
                    keys.insert(key);
                }
                debug_assert_eq!(
                    children.len(),
                    keys.len(),
                    "keyed siblings must each have a unique key"
                );
            };
            assert_unique_keys(old);
            assert_unique_keys(new);
        }

        // First up, we diff all the nodes with the same key at the beginning of the
        // children.
        //
        // `shared_prefix_count` is the count of how many nodes at the start of
        // `new` and `old` share the same keys.
        let (left_offset, right_offset) = match self.diff_keyed_ends(to, old, new, parent) {
            Some(count) => count,
            None => return,
        };

        // Ok, we now hopefully have a smaller range of children in the middle
        // within which to re-order nodes with the same keys, remove old nodes with
        // now-unused keys, and create new nodes with fresh keys.

        let old_middle = &old[left_offset..(old.len() - right_offset)];
        let new_middle = &new[left_offset..(new.len() - right_offset)];

        debug_assert!(
            !((old_middle.len() == new_middle.len()) && old_middle.is_empty()),
            "keyed children must have the same number of children"
        );

        if new_middle.is_empty() {
            // remove the old elements
            self.remove_nodes(to, old_middle, None);
        } else if old_middle.is_empty() {
            // there were no old elements, so just create the new elements
            // we need to find the right "foothold" though - we shouldn't use the "append" at all
            if left_offset == 0 {
                // insert at the beginning of the old list
                let foothold = &old[old.len() - right_offset];
                self.create_and_insert_before(to, new_middle, foothold, parent);
            } else if right_offset == 0 {
                // insert at the end  the old list
                let foothold = old.last().unwrap();
                self.create_and_insert_after(to, new_middle, foothold, parent);
            } else {
                // inserting in the middle
                let foothold = &old[left_offset - 1];
                self.create_and_insert_after(to, new_middle, foothold, parent);
            }
        } else {
            self.diff_keyed_middle(to, old_middle, new_middle, parent);
        }
    }

    /// Diff both ends of the children that share keys.
    ///
    /// Returns a left offset and right offset of that indicates a smaller section to pass onto the middle diffing.
    ///
    /// If there is no offset, then this function returns None and the diffing is complete.
    fn diff_keyed_ends(
        &mut self,
        to: &mut impl WriteMutations,
        old: &[VNode],
        new: &[VNode],
        parent: Option<ElementRef>,
    ) -> Option<(usize, usize)> {
        let mut left_offset = 0;

        for (old, new) in old.iter().zip(new.iter()) {
            // abort early if we finally run into nodes with different keys
            if old.key != new.key {
                break;
            }
            old.diff_node(new, self, to);
            left_offset += 1;
        }

        // If that was all of the old children, then create and append the remaining
        // new children and we're finished.
        if left_offset == old.len() {
            self.create_and_insert_after(to, &new[left_offset..], old.last().unwrap(), parent);
            return None;
        }

        // And if that was all of the new children, then remove all of the remaining
        // old children and we're finished.
        if left_offset == new.len() {
            self.remove_nodes(to, &old[left_offset..], None);
            return None;
        }

        // if the shared prefix is less than either length, then we need to walk backwards
        let mut right_offset = 0;
        for (old, new) in old.iter().rev().zip(new.iter().rev()) {
            // abort early if we finally run into nodes with different keys
            if old.key != new.key {
                break;
            }
            old.diff_node(new, self, to);
            right_offset += 1;
        }

        Some((left_offset, right_offset))
    }

    // The most-general, expensive code path for keyed children diffing.
    //
    // We find the longest subsequence within `old` of children that are relatively
    // ordered the same way in `new` (via finding a longest-increasing-subsequence
    // of the old child's index within `new`). The children that are elements of
    // this subsequence will remain in place, minimizing the number of DOM moves we
    // will have to do.
    //
    // Upon entry to this function, the change list stack must be empty.
    //
    // This function will load the appropriate nodes onto the stack and do diffing in place.
    //
    // Upon exit from this function, it will be restored to that same self.
    #[allow(clippy::too_many_lines)]
    fn diff_keyed_middle(
        &mut self,
        to: &mut impl WriteMutations,
        old: &[VNode],
        new: &[VNode],
        parent: Option<ElementRef>,
    ) {
        /*
        1. Map the old keys into a numerical ordering based on indices.
        2. Create a map of old key to its index
        3. Map each new key to the old key, carrying over the old index.
            - IE if we have ABCD becomes BACD, our sequence would be 1,0,2,3
            - if we have ABCD to ABDE, our sequence would be 0,1,3,MAX because E doesn't exist

        now, we should have a list of integers that indicates where in the old list the new items mapto.

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
        debug_assert_ne!(new.first().map(|i| &i.key), old.first().map(|i| &i.key));
        debug_assert_ne!(new.last().map(|i| &i.key), old.last().map(|i| &i.key));

        // 1. Map the old keys into a numerical ordering based on indices.
        // 2. Create a map of old key to its index
        // IE if the keys were A B C, then we would have (A, 1) (B, 2) (C, 3).
        let old_key_to_old_index = old
            .iter()
            .enumerate()
            .map(|(i, o)| (o.key.as_ref().unwrap(), i))
            .collect::<FxHashMap<_, _>>();

        let mut shared_keys = FxHashSet::default();

        // 3. Map each new key to the old key, carrying over the old index.
        let new_index_to_old_index = new
            .iter()
            .map(|node| {
                let key = node.key.as_ref().unwrap();
                if let Some(&index) = old_key_to_old_index.get(&key) {
                    shared_keys.insert(key);
                    index
                } else {
                    u32::MAX as usize
                }
            })
            .collect::<Vec<_>>();

        // If none of the old keys are reused by the new children, then we remove all the remaining old children and
        // create the new children afresh.
        if shared_keys.is_empty() {
            if !old.is_empty() {
                let m = self.create_children(to, new, parent);
                self.remove_nodes(to, old, Some(m));
            } else {
                // I think this is wrong - why are we appending?
                // only valid of the if there are no trailing elements
                // self.create_and_append_children(new);

                todo!("we should never be appending - just creating N");
            }
            return;
        }

        // remove any old children that are not shared
        // todo: make this an iterator
        for child in old {
            let key = child.key.as_ref().unwrap();
            if !shared_keys.contains(&key) {
                child.remove_node(self, to, None, true);
            }
        }

        // 4. Compute the LIS of this list
        let mut lis_sequence = Vec::with_capacity(new_index_to_old_index.len());

        let mut predecessors = vec![0; new_index_to_old_index.len()];
        let mut starts = vec![0; new_index_to_old_index.len()];

        longest_increasing_subsequence::lis_with(
            &new_index_to_old_index,
            &mut lis_sequence,
            |a, b| a < b,
            &mut predecessors,
            &mut starts,
        );

        // the lis comes out backwards, I think. can't quite tell.
        lis_sequence.sort_unstable();

        // if a new node gets u32 max and is at the end, then it might be part of our LIS (because u32 max is a valid LIS)
        if lis_sequence.last().map(|f| new_index_to_old_index[*f]) == Some(u32::MAX as usize) {
            lis_sequence.pop();
        }

        for idx in &lis_sequence {
            old[new_index_to_old_index[*idx]].diff_node(&new[*idx], self, to);
        }

        let mut nodes_created = 0;

        // add mount instruction for the first items not covered by the lis
        let last = *lis_sequence.last().unwrap();
        if last < (new.len() - 1) {
            for (idx, new_node) in new[(last + 1)..].iter().enumerate() {
                let new_idx = idx + last + 1;
                let old_index = new_index_to_old_index[new_idx];
                if old_index == u32::MAX as usize {
                    nodes_created += new_node.create(self, to, parent);
                } else {
                    old[old_index].diff_node(new_node, self, to);
                    nodes_created += new_node.push_all_real_nodes(self, to);
                }
            }

            let id = new[last].find_last_element(self);
            if nodes_created > 0 {
                to.insert_nodes_after(id, nodes_created)
            }
            nodes_created = 0;
        }

        // for each spacing, generate a mount instruction
        let mut lis_iter = lis_sequence.iter().rev();
        let mut last = *lis_iter.next().unwrap();
        for next in lis_iter {
            if last - next > 1 {
                for (idx, new_node) in new[(next + 1)..last].iter().enumerate() {
                    let new_idx = idx + next + 1;
                    let old_index = new_index_to_old_index[new_idx];
                    if old_index == u32::MAX as usize {
                        nodes_created += new_node.create(self, to, parent);
                    } else {
                        old[old_index].diff_node(new_node, self, to);
                        nodes_created += new_node.push_all_real_nodes(self, to);
                    }
                }

                let id = new[last].find_first_element(self);
                if nodes_created > 0 {
                    to.insert_nodes_before(id, nodes_created);
                }

                nodes_created = 0;
            }
            last = *next;
        }

        // add mount instruction for the last items not covered by the lis
        let first_lis = *lis_sequence.first().unwrap();
        if first_lis > 0 {
            for (idx, new_node) in new[..first_lis].iter().enumerate() {
                let old_index = new_index_to_old_index[idx];
                if old_index == u32::MAX as usize {
                    nodes_created += new_node.create(self, to, parent);
                } else {
                    old[old_index].diff_node(new_node, self, to);
                    nodes_created += new_node.push_all_real_nodes(self, to);
                }
            }

            let id = new[first_lis].find_first_element(self);
            if nodes_created > 0 {
                to.insert_nodes_before(id, nodes_created);
            }
        }
    }

    fn create_and_insert_before(
        &mut self,
        to: &mut impl WriteMutations,
        new: &[VNode],
        before: &VNode,
        parent: Option<ElementRef>,
    ) {
        let m = self.create_children(to, new, parent);
        let id = before.find_first_element(self);
        to.insert_nodes_before(id, m);
    }

    fn create_and_insert_after(
        &mut self,
        to: &mut impl WriteMutations,
        new: &[VNode],
        after: &VNode,
        parent: Option<ElementRef>,
    ) {
        let m = self.create_children(to, new, parent);
        let id = after.find_last_element(self);
        to.insert_nodes_after(id, m);
    }
}

impl VNode {
    /// Push all the real nodes on the stack
    pub(crate) fn push_all_real_nodes(
        &self,
        dom: &VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        let template = self.template.get();

        let mount = dom.mounts.get(self.mount.get().0).unwrap();

        template
            .roots
            .iter()
            .enumerate()
            .map(|(root_idx, _)| match &self.template.get().roots[root_idx] {
                TemplateNode::Dynamic { id: idx } => match &self.dynamic_nodes[*idx] {
                    DynamicNode::Placeholder(_) | DynamicNode::Text(_) => {
                        to.push_root(mount.root_ids[root_idx]);
                        1
                    }
                    DynamicNode::Fragment(nodes) => {
                        let mut accumulated = 0;
                        for node in nodes {
                            accumulated += node.push_all_real_nodes(dom, to);
                        }
                        accumulated
                    }
                    DynamicNode::Component(_) => {
                        let scope = ScopeId(mount.mounted_dynamic_nodes[*idx]);
                        let node = dom.get_scope(scope).unwrap().root_node();
                        node.push_all_real_nodes(dom, to)
                    }
                },
                _ => {
                    to.push_root(mount.root_ids[root_idx]);
                    1
                }
            })
            .sum()
    }
}
