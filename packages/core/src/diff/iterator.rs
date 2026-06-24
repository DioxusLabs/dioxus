//! Fragment child reconciliation.
//!
//! Invariants maintained here:
//! - Non-empty fragment diffs always write one mount per new child into the fragment child writer
//!   range.

use crate::{
    DynamicNode, ElementId, VNodeChild, VirtualDom,
    diff::{
        context::{DiffFrame, DiffState},
        placement::{InsertionSite, at_site, create_at_site_with_mounts},
    },
    innerlude::{MountId, WriteMutations},
    mount::FragmentMountWriter,
    nodes::VNode,
};

use rustc_hash::FxHashMap;
use std::ops::Range;

struct FragmentPlacementPlan {
    new_to_old: Vec<Option<usize>>,
    stable: Vec<bool>,
}

impl FragmentPlacementPlan {
    fn new(len: usize) -> Self {
        Self {
            new_to_old: vec![None; len],
            stable: vec![false; len],
        }
    }

    fn reuse(&mut self, new_index: usize, old_index: usize, old: &VNode, new: &VNode) {
        self.new_to_old[new_index] = Some(old_index);
        self.stable[new_index] = old.template() == new.template();
    }

    fn placement_runs(&self, live_stable: &[bool]) -> Vec<Range<usize>> {
        let mut runs = Vec::new();
        let mut run_start = None;
        let mut run_has_placed_child = false;

        for (index, (&live, &stable)) in live_stable.iter().zip(&self.stable).enumerate() {
            if live {
                if let Some(start) = run_start.take()
                    && run_has_placed_child
                {
                    runs.push(start..index);
                }
                run_has_placed_child = false;
                continue;
            }

            if !stable {
                run_start.get_or_insert(index);
                run_has_placed_child = true;
            }
        }

        if let Some(start) = run_start
            && run_has_placed_child
        {
            runs.push(start..self.stable.len());
        }

        runs
    }
}

struct StableFragmentEdges {
    live_stable: Vec<bool>,
    next_first: Vec<Option<ElementId>>,
    prev_last: Vec<Option<ElementId>>,
}

impl StableFragmentEdges {
    fn new(
        new: &[VNode],
        new_mounts: &[Option<MountId>],
        stable: &[bool],
        dom: &VirtualDom,
    ) -> Self {
        let mut first_edges = vec![None; new.len()];
        let mut last_edges = vec![None; new.len()];
        let mut live_stable = vec![false; new.len()];

        for idx in 0..new.len() {
            if !stable[idx] {
                continue;
            }
            let Some(mount) = new_mounts[idx] else {
                continue;
            };
            first_edges[idx] = new[idx].find_first_element(mount, dom);
            last_edges[idx] = new[idx].find_last_element(mount, dom);
            live_stable[idx] = first_edges[idx].is_some() || last_edges[idx].is_some();
        }

        let mut next_first = vec![None; new.len() + 1];
        let mut next = None;
        for idx in (0..new.len()).rev() {
            if live_stable[idx] {
                next = first_edges[idx];
            }
            next_first[idx] = next;
        }

        let mut prev_last = vec![None; new.len() + 1];
        let mut prev = None;
        for idx in 0..new.len() {
            prev_last[idx] = prev;
            if live_stable[idx] {
                prev = last_edges[idx];
            }
        }
        prev_last[new.len()] = prev;

        Self {
            live_stable,
            next_first,
            prev_last,
        }
    }

    fn next_first(&self, boundary: usize) -> Option<ElementId> {
        self.next_first[boundary]
    }

    fn prev_last(&self, boundary: usize) -> Option<ElementId> {
        self.prev_last[boundary]
    }
}

/// The shared inputs threaded through fragment placement: the old/new sibling slices, their
/// mounts, the parent mount, and where resulting child mounts are written.
#[derive(Clone, Copy)]
struct FragmentInputs<'a> {
    old: &'a [VNode],
    old_mounts: &'a [MountId],
    new: &'a [VNode],
    parent: Option<MountId>,
    new_children: FragmentMountWriter,
    new_offset: usize,
}

impl DiffState<'_, '_, '_, '_> {
    /// Diff two non-empty fragment child lists.
    ///
    /// Invariant: both lists are internally homogeneous with respect to keys: either every child is
    /// keyed or no child is keyed.
    pub(crate) fn diff_non_empty_fragment(
        &mut self,
        old: &[VNode],
        old_mounts: &[MountId],
        new: &[VNode],
        parent: Option<MountId>,
        new_children: FragmentMountWriter,
        fallback_site: Option<InsertionSite>,
    ) {
        dioxus_debug_assert!(
            new_children.len() == new.len(),
            "fragment child writer range must match the new child list"
        );
        let new_is_keyed = new[0].key().is_some();
        let old_is_keyed = old[0].key().is_some();
        dioxus_debug_assert!(
            new.iter().all(|n| n.key().is_some() == new_is_keyed),
            "all siblings must be keyed or all siblings must be non-keyed"
        );
        dioxus_debug_assert!(
            old.iter().all(|o| o.key().is_some() == old_is_keyed),
            "all siblings must be keyed or all siblings must be non-keyed"
        );

        if new_is_keyed && old_is_keyed {
            self.diff_keyed_children(old, old_mounts, new, parent, new_children, fallback_site)
        } else {
            self.diff_non_keyed_children(old, old_mounts, new, parent, new_children, fallback_site)
        }
    }

    /// Diff children that are not keyed.
    ///
    /// Invariant: pair indices preserve child identity.
    fn diff_non_keyed_children(
        &mut self,
        old: &[VNode],
        old_mounts: &[MountId],
        new: &[VNode],
        parent: Option<MountId>,
        new_children: FragmentMountWriter,
        fallback_site: Option<InsertionSite>,
    ) {
        // Handled these cases in `diff_children` before calling this function.
        dioxus_debug_assert!(!new.is_empty());
        dioxus_debug_assert!(!old.is_empty());

        let paired = old.len().min(new.len());
        let mut plan = FragmentPlacementPlan::new(new.len());

        for idx in 0..paired {
            plan.reuse(idx, idx, &old[idx], &new[idx]);
        }

        self.execute_fragment_plan(
            FragmentInputs {
                old,
                old_mounts,
                new,
                parent,
                new_children,
                new_offset: 0,
            },
            plan,
            fallback_site,
        );

        if old.len() > new.len() {
            self.dom.remove_nodes(
                self.to.as_deref_mut(),
                &old[new.len()..],
                &old_mounts[new.len()..],
            );
        }
    }

    /// Diff keyed children.
    ///
    /// Invariant: keys are unique within each sibling list.
    fn diff_keyed_children(
        &mut self,
        old: &[VNode],
        old_mounts: &[MountId],
        new: &[VNode],
        parent: Option<MountId>,
        new_children: FragmentMountWriter,
        fallback_site: Option<InsertionSite>,
    ) {
        #[cfg(debug_assertions)]
        {
            let assert_unique_keys = |children: &[VNode]| {
                let mut keys = rustc_hash::FxHashSet::default();
                for child in children {
                    let key = child.key();
                    dioxus_debug_assert!(
                        key.is_some(),
                        "if any sibling is keyed, all siblings must be keyed"
                    );
                    keys.insert(key);
                }
                dioxus_debug_assert!(
                    children.len() == keys.len(),
                    "keyed siblings must each have a unique key"
                );
            };
            assert_unique_keys(old);
            assert_unique_keys(new);
        }

        // Peel the shared prefix and suffix so the key map and LIS only cover the
        // middle that actually reordered. Edits confined to the ends - append,
        // prepend, truncation, in-place updates, or a localized reorder inside a
        // large stable list - never build a full-length key map.
        let min_len = old.len().min(new.len());
        let mut prefix = 0;
        while prefix < min_len && old[prefix].key() == new[prefix].key() {
            prefix += 1;
        }
        let mut suffix = 0;
        while suffix < min_len - prefix
            && old[old.len() - 1 - suffix].key() == new[new.len() - 1 - suffix].key()
        {
            suffix += 1;
        }

        let old_suffix_start = old.len() - suffix;
        let new_suffix_start = new.len() - suffix;

        let mut plan = FragmentPlacementPlan::new(new.len());
        let mut old_is_shared = vec![false; old.len()];

        for idx in 0..prefix {
            plan.reuse(idx, idx, &old[idx], &new[idx]);
            old_is_shared[idx] = true;
        }

        for offset in 0..suffix {
            let old_idx = old_suffix_start + offset;
            let new_idx = new_suffix_start + offset;
            plan.reuse(new_idx, old_idx, &old[old_idx], &new[new_idx]);
            old_is_shared[old_idx] = true;
        }

        if prefix < old_suffix_start && prefix < new_suffix_start {
            let old_key_to_old_index = old[prefix..old_suffix_start]
                .iter()
                .enumerate()
                .map(|(i, o)| (o.key().unwrap(), prefix + i))
                .collect::<FxHashMap<_, _>>();

            let mut shared_middle_indices = Vec::new();
            let mut shared_old_indices = Vec::new();
            let new_index_to_old_index = new[prefix..new_suffix_start]
                .iter()
                .enumerate()
                .map(|(middle_new_idx, node)| {
                    let key = node.key().unwrap();
                    if let Some(&index) = old_key_to_old_index.get(key) {
                        if !old_is_shared[index] {
                            old_is_shared[index] = true;
                            shared_middle_indices.push(middle_new_idx);
                            shared_old_indices.push(index);
                        }
                        Some(index)
                    } else {
                        None
                    }
                })
                .collect::<Box<[_]>>();

            let mut in_lis = vec![false; new_index_to_old_index.len()];
            if !shared_old_indices.is_empty() {
                let mut lis_sequence = Vec::with_capacity(shared_old_indices.len());
                let mut allocation = vec![0; shared_old_indices.len() * 2];
                let (predecessors, starts) = allocation.split_at_mut(shared_old_indices.len());

                longest_increasing_subsequence::lis_with(
                    &shared_old_indices,
                    &mut lis_sequence,
                    |a, b| a < b,
                    predecessors,
                    starts,
                );

                for idx in lis_sequence {
                    in_lis[shared_middle_indices[idx]] = true;
                }
            }

            for (middle_new_idx, old_index) in new_index_to_old_index.iter().copied().enumerate() {
                let Some(old_index) = old_index else { continue };
                let new_idx = prefix + middle_new_idx;
                plan.reuse(new_idx, old_index, &old[old_index], &new[new_idx]);
                plan.stable[new_idx] = plan.stable[new_idx] && in_lis[middle_new_idx];
            }
        }

        self.execute_fragment_plan(
            FragmentInputs {
                old,
                old_mounts,
                new,
                parent,
                new_children,
                new_offset: 0,
            },
            plan,
            fallback_site,
        );

        for (_, (child_to_remove, mount_to_remove)) in old
            .iter()
            .zip(old_mounts)
            .enumerate()
            .filter(|(old_index, _)| !old_is_shared[*old_index])
        {
            child_to_remove.remove_node(*mount_to_remove, self.dom, self.to.as_deref_mut());
        }
    }

    fn execute_fragment_plan(
        &mut self,
        inputs: FragmentInputs,
        plan: FragmentPlacementPlan,
        fallback_site: Option<InsertionSite>,
    ) {
        let FragmentInputs {
            old,
            old_mounts,
            new,
            new_children,
            new_offset,
            ..
        } = inputs;
        let mut new_mounts = vec![None; new.len()];
        let mut anchorable = plan.stable.clone();

        for idx in 0..new.len() {
            if !plan.stable[idx] {
                continue;
            }
            let old_index = plan.new_to_old[idx].expect("stable child must be reused");
            let old_mount = old_mounts[old_index];
            self.dom
                .set_mounted_fragment_child(new_children, new_offset + idx, old_mount);
            let mount = DiffFrame::new(old_mount, &old[old_index], &new[idx]).diff_into(self);
            new_mounts[idx] = Some(mount);
            self.dom
                .set_mounted_fragment_child(new_children, new_offset + idx, mount);
            if mount != old_mount {
                anchorable[idx] = false;
            }
        }

        let stable_edges = StableFragmentEdges::new(new, &new_mounts, &anchorable, self.dom);
        for range in plan.placement_runs(&stable_edges.live_stable) {
            let site = self.has_writer().then(|| {
                stable_edges
                    .next_first(range.end)
                    .map(InsertionSite::before)
                    .or_else(|| {
                        stable_edges
                            .prev_last(range.start)
                            .map(InsertionSite::after)
                    })
                    .or(fallback_site)
                    .expect("visible fragment placement requires a fallback insertion site")
            });

            let context = self.context();
            let runtime = self.dom.runtime.clone();
            let dom = &mut *self.dom;
            let to = self.to.as_deref_mut();
            let mut replaced_nodes = Vec::new();
            if let Some(site) = site {
                let to = to.expect("writer checked");
                at_site(site, to, runtime, |to| {
                    let mut state = DiffState::new_with_context(dom, Some(to), context);
                    state.create_or_diff_placed_range(
                        &inputs,
                        &plan,
                        range.clone(),
                        &mut new_mounts,
                        &mut replaced_nodes,
                    )
                });
            } else {
                let mut state = DiffState::new_with_context(dom, to, context);
                state.create_or_diff_placed_range(
                    &inputs,
                    &plan,
                    range.clone(),
                    &mut new_mounts,
                    &mut replaced_nodes,
                );
            }

            for (node, mount) in replaced_nodes.into_iter().rev() {
                node.remove_node(mount, self.dom, self.to.as_deref_mut());
            }
        }

        for (idx, mount) in new_mounts.into_iter().enumerate() {
            let mount = mount.expect("fragment plan must materialize every new child");
            self.dom
                .set_mounted_fragment_child(new_children, new_offset + idx, mount);
        }
    }

    /// Materialize every non-stable child in a fragment placement range at the current renderer
    /// insertion site. Stable children inside the range are DOM-empty anchors in the logical order,
    /// so they are skipped while neighbouring placed children are spliced as one host segment.
    fn create_or_diff_placed_range<'a>(
        &mut self,
        inputs: &FragmentInputs<'a>,
        plan: &FragmentPlacementPlan,
        range: Range<usize>,
        new_mounts: &mut [Option<MountId>],
        replaced_nodes: &mut Vec<(&'a VNode, MountId)>,
    ) -> usize {
        let FragmentInputs {
            old,
            old_mounts,
            new,
            parent,
            new_children,
            new_offset,
        } = *inputs;
        for new_index in range.clone() {
            if plan.stable[new_index] {
                continue;
            }
            if let Some(old_index) = plan.new_to_old[new_index] {
                self.dom.set_mounted_fragment_child(
                    new_children,
                    new_offset + new_index,
                    old_mounts[old_index],
                );
            }
        }

        let mut nodes = 0;
        for new_index in range {
            if plan.stable[new_index] {
                continue;
            }
            let new_node = &new[new_index];
            let (created_nodes, mount) = if let Some(old_index) = plan.new_to_old[new_index] {
                let old_mount = old_mounts[old_index];
                let old_node = &old[old_index];
                let (nodes, mount) = if old_node.template() != new_node.template() {
                    let created =
                        new_node.create_mounted(self.dom, parent, parent, self.to.as_deref_mut());
                    replaced_nodes.push((old_node, old_mount));
                    (created.nodes, created.mount)
                } else {
                    let mount = DiffFrame::new(old_mount, old_node, new_node).diff_into(self);
                    let nodes = if self.has_writer() {
                        let to = self.to.as_deref_mut().expect("writer checked");
                        crate::MountedVNode::new(new_node, mount).push_all_root_nodes(self.dom, to)
                    } else {
                        0
                    };
                    (nodes, mount)
                };
                (nodes, mount)
            } else {
                let created =
                    new_node.create_mounted(self.dom, parent, parent, self.to.as_deref_mut());
                (created.nodes, created.mount)
            };
            new_mounts[new_index] = Some(mount);
            self.dom
                .set_mounted_fragment_child(new_children, new_offset + new_index, mount);
            nodes += created_nodes;
        }
        nodes
    }

    /// Create `new` under `parent`. When a writer is active the children are placed at `site`
    /// (computed lazily - no-writer diffs only materialize mount state and resolve no placement);
    /// otherwise only their mount state is created.
    pub(super) fn create_children_at_site(
        &mut self,
        new: &[VNode],
        parent: Option<MountId>,
        site: impl FnOnce(&mut Self) -> InsertionSite,
        children: FragmentMountWriter,
    ) -> usize {
        self.create_children_at_site_with_mounts(new, parent, site, |dom, idx, mount| {
            dom.set_mounted_fragment_child(children, idx, mount)
        })
    }

    fn create_children_at_site_with_mounts(
        &mut self,
        new: &[VNode],
        parent: Option<MountId>,
        site: impl FnOnce(&mut Self) -> InsertionSite,
        created_mount: impl FnMut(&mut VirtualDom, usize, MountId),
    ) -> usize {
        if self.has_writer() {
            let site = site(self);
            let to = self.to.as_deref_mut().expect("writer checked");
            create_at_site_with_mounts(new, parent, site, self.dom, to, created_mount)
        } else {
            self.dom.create_children_with_mounts(
                self.to.as_deref_mut(),
                new,
                parent,
                parent,
                created_mount,
            )
        }
    }
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
        for child in self.vnode().children() {
            match child {
                VNodeChild::Dynamic(anchor) => {
                    for slot in anchor.nodes() {
                        count +=
                            self.push_dynamic_root_node(slot.index(), mount, target_id, dom, to);
                    }
                }
                VNodeChild::Element(element) => {
                    if dom.mount_target_id(mount) == target_id
                        && let Some(anchor_idx) = element.anchor_index()
                        && let Some(id) = dom.mounted_anchor_node(mount, anchor_idx)
                    {
                        count += push_live_root(to, id.element_id());
                    }
                }
                VNodeChild::Text(text) => {
                    if dom.mount_target_id(mount) == target_id
                        && let Some(anchor_idx) = text.anchor_index()
                        && let Some(id) = dom.mounted_anchor_node(mount, anchor_idx)
                    {
                        count += push_live_root(to, id.element_id());
                    }
                }
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
        match &self.vnode().dynamic_node_values()[idx] {
            DynamicNode::Fragment(nodes) => {
                let mounts = dom.mounted_fragment_children_exact(mount, idx, nodes.len());
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
                .expect("scope output")
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
    // read from a root anchor/dynamic text slot for a vnode whose mount target already matches
    // `target_id`, so the live element id has been allocated in that target by
    // `load_template_root` / `assign_node_id`.
    to.push_id(id);
    1
}
