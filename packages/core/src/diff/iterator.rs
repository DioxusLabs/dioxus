use crate::{
    DynamicNode, ElementId, VirtualDom,
    diff::{
        context::{DiffFrame, DiffState},
        placement::{ElementEdge, at_site, create_at_site, insertion_site_at},
    },
    innerlude::{MountId, MountRef, WriteMutations},
    mutations::reborrow_writer,
    nodes::VNode,
};

use rustc_hash::{FxHashMap, FxHashSet};

impl DiffState<'_, '_, '_> {
    pub(crate) fn diff_non_empty_fragment(
        &mut self,
        old: &[VNode],
        old_mounts: &[MountId],
        new: &[VNode],
        parent: Option<MountRef>,
    ) -> Vec<MountId> {
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
            self.diff_keyed_children(old, old_mounts, new, parent)
        } else {
            self.diff_non_keyed_children(old, old_mounts, new, parent)
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
        old_mounts: &[MountId],
        new: &[VNode],
        parent: Option<MountRef>,
    ) -> Vec<MountId> {
        use std::cmp::Ordering;

        // Handled these cases in `diff_children` before calling this function.
        dioxus_debug_assert!(!new.is_empty());
        dioxus_debug_assert!(!old.is_empty());

        let mut new_mounts = vec![None; new.len()];
        match old.len().cmp(&new.len()) {
            Ordering::Greater => self.dom.remove_nodes(
                reborrow_writer(&mut self.to),
                &old[new.len()..],
                &old_mounts[new.len()..],
            ),
            Ordering::Less => {
                let created = self.create_and_insert(
                    ElementEdge::Last,
                    &new[old.len()..],
                    old.last().unwrap(),
                    old_mounts[old.len() - 1],
                    parent,
                );
                for (slot, mount) in new_mounts[old.len()..].iter_mut().zip(created.mounts) {
                    *slot = Some(mount);
                }
            }
            Ordering::Equal => {}
        }

        self.diff_child_pairs(old, old_mounts, new, &mut new_mounts, 0);
        new_mounts
            .into_iter()
            .map(|mount| mount.expect("new child should have a mount after non-keyed diff"))
            .collect()
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
        old: &[VNode],
        old_mounts: &[MountId],
        new: &[VNode],
        parent: Option<MountRef>,
    ) -> Vec<MountId> {
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
            let first_old = old.first().unwrap();
            let site = insertion_site_at(
                ElementEdge::First,
                crate::MountedVNode::new(first_old, old_mounts[0]),
                &[],
                self.dom,
                self.context(),
            );
            let created =
                create_at_site(new, parent, site, self.dom, reborrow_writer(&mut self.to));
            self.dom
                .remove_nodes(reborrow_writer(&mut self.to), old, old_mounts);
            return created.mounts;
        }

        for (child_to_remove, mount_to_remove) in old
            .iter()
            .zip(old_mounts)
            .filter(|(child, _)| !shared_keys.contains(child.key.as_ref().unwrap()))
        {
            child_to_remove.remove_node(*mount_to_remove, self.dom, reborrow_writer(&mut self.to));
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

        let mut new_mounts = vec![None; new.len()];
        for idx in &lis_sequence {
            let old_index = new_index_to_old_index[*idx];
            let old_node = &old[old_index];
            let mount = DiffFrame::new(old_mounts[old_index], old_node, &new[*idx]).diff_into(self);
            new_mounts[*idx] = Some(mount);
        }

        let last = *lis_sequence.first().unwrap();
        if last < (new.len() - 1) {
            self.splice_around_diffing(
                ElementEdge::Last,
                new,
                old,
                old_mounts,
                last,
                parent,
                &new_index_to_old_index,
                (last + 1)..new.len(),
                &mut new_mounts,
            );
        }

        for pair in lis_sequence.windows(2) {
            let (last, next) = (pair[0], pair[1]);
            if last - next > 1 {
                self.splice_around_diffing(
                    ElementEdge::First,
                    new,
                    old,
                    old_mounts,
                    last,
                    parent,
                    &new_index_to_old_index,
                    (next + 1)..last,
                    &mut new_mounts,
                );
            }
        }

        let first_lis = *lis_sequence.last().unwrap();
        if first_lis > 0 {
            self.splice_around_diffing(
                ElementEdge::First,
                new,
                old,
                old_mounts,
                first_lis,
                parent,
                &new_index_to_old_index,
                0..first_lis,
                &mut new_mounts,
            );
        }

        new_mounts
            .into_iter()
            .map(|mount| mount.expect("new child should have a mount after keyed diff"))
            .collect()
    }

    fn diff_child_pairs(
        &mut self,
        old: &[VNode],
        old_mounts: &[MountId],
        new: &[VNode],
        new_mounts: &mut [Option<MountId>],
        new_offset: usize,
    ) {
        let len = old.len().min(new.len());
        for idx in (0..len).rev() {
            let old = &old[idx];
            let new = &new[idx];
            let mount = DiffFrame::new(old_mounts[idx], old, new).diff_into(self);
            new_mounts[new_offset + idx] = Some(mount);
        }
    }

    fn splice_around_diffing(
        &mut self,
        edge: ElementEdge,
        new: &[VNode],
        old: &[VNode],
        old_mounts: &[MountId],
        sibling_idx: usize,
        parent: Option<MountRef>,
        new_index_to_old_index: &[usize],
        range: std::ops::Range<usize>,
        new_mounts: &mut [Option<MountId>],
    ) {
        let skip = collect_splice_mounts(old_mounts, new_index_to_old_index, range.clone());
        let context = self.context();
        let sibling_mount = new_mounts[sibling_idx].expect("sibling should already be diffed");
        let site = insertion_site_at(
            edge,
            crate::MountedVNode::new(&new[sibling_idx], sibling_mount),
            &skip,
            self.dom,
            context,
        );
        let runtime = self.dom.runtime.clone();
        let dom = &mut *self.dom;
        let to = reborrow_writer(&mut self.to);
        at_site(site, to, runtime, |to| {
            let mut state = DiffState::new_with_context(dom, to, context);
            state.create_or_diff_range(
                new,
                old,
                old_mounts,
                parent,
                new_index_to_old_index,
                range,
                new_mounts,
            )
        });
    }

    fn create_or_diff_range(
        &mut self,
        new: &[VNode],
        old: &[VNode],
        old_mounts: &[MountId],
        parent: Option<MountRef>,
        new_index_to_old_index: &[usize],
        range: std::ops::Range<usize>,
        new_mounts: &mut [Option<MountId>],
    ) -> usize {
        let range_start = range.start;
        let mut nodes = 0;
        for (idx, new_node) in new[range.clone()].iter().enumerate() {
            let new_index = range_start + idx;
            let old_index = new_index_to_old_index[range_start + idx];
            let (created_nodes, mount) = if let Some(old_node) = old.get(old_index) {
                let mount =
                    DiffFrame::new(old_mounts[old_index], old_node, new_node).diff_into(self);
                let nodes = if let Some(to) = reborrow_writer(&mut self.to) {
                    crate::MountedVNode::new(new_node, mount).push_all_root_nodes(self.dom, to)
                } else {
                    0
                };
                (nodes, mount)
            } else {
                let created = new_node.create_with_parents(
                    self.dom,
                    parent,
                    parent,
                    reborrow_writer(&mut self.to),
                );
                (created.nodes, created.mount)
            };
            new_mounts[new_index] = Some(mount);
            nodes += created_nodes;
        }
        nodes
    }

    fn create_and_insert(
        &mut self,
        edge: ElementEdge,
        new: &[VNode],
        sibling: &VNode,
        sibling_mount: MountId,
        parent: Option<MountRef>,
    ) -> crate::diff::CreatedNodes {
        let site = insertion_site_at(
            edge,
            crate::MountedVNode::new(sibling, sibling_mount),
            &[],
            self.dom,
            self.context(),
        );
        create_at_site(new, parent, site, self.dom, reborrow_writer(&mut self.to))
    }
}

fn collect_splice_mounts(
    old_mounts: &[MountId],
    new_index_to_old_index: &[usize],
    range: std::ops::Range<usize>,
) -> Vec<MountId> {
    // Each splice range is the *non-LIS* portion of the keyed middle, so the
    // new sibling at `range` is not yet claimed; only the matching old vnode's
    // live mount needs to be added to the skip list so placement lookups don't
    // try to use a sibling that's about to be moved. The non-LIS old entries
    // come straight from the previous render with their mounts intact — no
    // earlier diff step has called `claim_mount` on them yet — so the
    // mount is always live by the time we collect it.
    range
        .filter_map(|idx| old_mounts.get(new_index_to_old_index[idx]).copied())
        .collect()
}

impl crate::MountedVNode<'_> {
    /// Push all the root nodes on the stack
    pub(crate) fn push_all_root_nodes(
        self,
        dom: &VirtualDom,
        to: &mut dyn WriteMutations,
    ) -> usize {
        let mount = self.mount();
        let target_id = dom.current_render_target_id();

        let mut count = 0;
        for (root_idx, static_op, dynamic_anchor) in self.template.root_slots() {
            if let Some(anchor) = dynamic_anchor {
                count +=
                    self.push_dynamic_root_node(anchor.value_start(), mount, target_id, dom, to);
                continue;
            }

            debug_assert!(static_op.is_some());
            if dom.mount_target_id(mount) == target_id
                && let Some(id) = dom.mounted_root_node(mount, root_idx)
            {
                count += push_live_root(to, id.element_id());
            }
        }

        count
    }

    fn push_dynamic_root_node(
        self,
        idx: usize,
        mount: MountId,
        target_id: crate::RenderTargetId,
        dom: &VirtualDom,
        to: &mut dyn WriteMutations,
    ) -> usize {
        match self.dynamic_values[idx].node() {
            DynamicNode::Fragment(nodes) => {
                let mounts = dom.mounted_fragment_children(mount, idx, nodes.len());
                nodes
                    .iter()
                    .zip(mounts)
                    .map(|(node, mount)| {
                        crate::MountedVNode::new(node, mount).push_all_root_nodes(dom, to)
                    })
                    .sum()
            }
            DynamicNode::Component(_) => dom
                .get_scope(dom.unchecked_mounted_dynamic_component_scope(mount, idx))
                .unwrap()
                .try_mounted_root_node()
                .expect("component scope should have mounted output")
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
