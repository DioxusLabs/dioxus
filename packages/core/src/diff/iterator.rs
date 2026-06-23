//! Fragment child reconciliation.
//!
//! Invariants maintained here:
//! - Non-empty fragment diffs always write one mount per new child into the pending parent range.
//! - Pairwise diffs run in document order so earlier replacements can still use later committed
//!   siblings as placement anchors.
//! - Keyed removals are delayed until every splice has selected its insertion site.
//! - `usize::MAX` in `new_index_to_old_index` is the only marker for a newly created keyed child;
//!   every other index must point into the old sibling list.
//! - Keyed lists whose edits are confined to one end (append, prepend, truncation, or no change)
//!   skip the key map and LIS entirely: the shared prefix/suffix is diffed in place and the single
//!   remaining gap is a bulk insert or remove. Only genuine middle reorders pay for the key map.

use crate::{
    DynamicNode, ElementId, VNodeChild, VirtualDom,
    diff::{
        context::{DiffFrame, DiffState},
        placement::{
            ElementEdge, InsertionSite, at_site, create_at_site_with_mounts, insertion_site_at,
            vnode_edge_site,
        },
    },
    innerlude::{MountId, MountRef, WriteMutations},
    mount::FragmentMountWriter,
    nodes::VNode,
};

use rustc_hash::FxHashMap;

impl DiffState<'_, '_, '_, '_> {
    /// Diff two non-empty fragment child lists.
    ///
    /// Invariant: both lists are internally homogeneous with respect to keys: either every child is
    /// keyed or no child is keyed. Empty-list transitions are handled by the caller.
    pub(crate) fn diff_non_empty_fragment(
        &mut self,
        old: &[VNode],
        old_mounts: &[MountId],
        new: &[VNode],
        parent: Option<MountRef>,
        new_children: FragmentMountWriter,
    ) {
        dioxus_debug_assert!(
            new_children.len() == new.len(),
            "pending fragment range must match the new child list"
        );
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
            self.diff_keyed_children(old, old_mounts, new, parent, new_children)
        } else {
            self.diff_non_keyed_children(old, old_mounts, new, parent, new_children)
        }
    }

    /// Diff children that are not keyed.
    ///
    /// Invariant: pair indices preserve child identity. Tail removals stay mounted until after all
    /// paired children have diffed, because paired replacements may still need tail siblings as
    /// placement anchors.
    fn diff_non_keyed_children(
        &mut self,
        old: &[VNode],
        old_mounts: &[MountId],
        new: &[VNode],
        parent: Option<MountRef>,
        new_children: FragmentMountWriter,
    ) {
        // Handled these cases in `diff_children` before calling this function.
        dioxus_debug_assert!(!new.is_empty());
        dioxus_debug_assert!(!old.is_empty());

        let paired = old.len().min(new.len());
        let mut new_mounts = Vec::with_capacity(new.len());
        for idx in 0..paired {
            let mount = DiffFrame::new(old_mounts[idx], &old[idx], &new[idx]).diff_into(self);
            new_mounts.push(mount);
            self.dom
                .set_mounted_fragment_child(new_children, idx, mount);
        }

        if old.len() < new.len() {
            // Insert the new tail after the last paired child. Anchor on the
            // freshly diffed *new* child, not the old one: an old child with no
            // live DOM (e.g. an empty fragment) offers no insertion edge, so
            // anchoring on it would lose the tail to the document root. The
            // already-diffed new child exposes its current content's edge, and
            // diffing the pairs first means an empty leading child's own content
            // is placed before the tail rather than after it.
            let anchor_idx = paired - 1;
            self.create_and_insert(
                ElementEdge::Last,
                &new[old.len()..],
                &new[anchor_idx],
                new_mounts[anchor_idx],
                parent,
                |dom, offset, mount| {
                    new_mounts.push(mount);
                    dom.set_mounted_fragment_child(new_children, old.len() + offset, mount);
                },
            );
        } else if old.len() > new.len() {
            // Removed tail children stayed mounted through the pair diffs above
            // so paired replacements could anchor against them; remove them now.
            // The committed parent fragment mount list is not replaced until this
            // fragment diff returns.
            self.dom.remove_nodes(
                self.to.as_deref_mut(),
                &old[new.len()..],
                &old_mounts[new.len()..],
            );
        }
    }

    /// Diff the shared prefix and suffix pairs in place, before any removals, so a pair whose
    /// template changed can still anchor against its live neighbours.
    fn diff_shared_ends(
        &mut self,
        old: &[VNode],
        old_mounts: &[MountId],
        new: &[VNode],
        new_mounts: &mut [Option<MountId>],
        new_children: FragmentMountWriter,
        prefix: usize,
        old_suffix_start: usize,
        new_suffix_start: usize,
    ) {
        self.diff_child_pairs(
            &old[..prefix],
            &old_mounts[..prefix],
            &new[..prefix],
            new_mounts,
            new_children,
            0,
        );
        self.diff_child_pairs(
            &old[old_suffix_start..],
            &old_mounts[old_suffix_start..],
            &new[new_suffix_start..],
            new_mounts,
            new_children,
            new_suffix_start,
        );
    }

    /// Diff keyed children.
    ///
    /// Invariant: keys are unique within each sibling list. Shared keyed children keep their old
    /// mount unless their vnode replacement requires a new mount; newly keyed children are marked
    /// with `usize::MAX` until materialized.
    fn diff_keyed_children(
        &mut self,
        old: &[VNode],
        old_mounts: &[MountId],
        new: &[VNode],
        parent: Option<MountRef>,
        new_children: FragmentMountWriter,
    ) {
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
        while prefix < min_len && old[prefix].key == new[prefix].key {
            prefix += 1;
        }
        let mut suffix = 0;
        while suffix < min_len - prefix
            && old[old.len() - 1 - suffix].key == new[new.len() - 1 - suffix].key
        {
            suffix += 1;
        }

        let old_suffix_start = old.len() - suffix;
        let new_suffix_start = new.len() - suffix;

        let mut new_mounts = vec![None; new.len()];
        let pure_insert = prefix + suffix == old.len() && prefix + suffix != new.len();
        let pure_insert_anchor_keeps_template = pure_insert
            && if suffix > 0 {
                old[old_suffix_start].template == new[new_suffix_start].template
            } else {
                old[prefix - 1].template == new[prefix - 1].template
            };

        if !pure_insert || pure_insert_anchor_keeps_template {
            self.diff_shared_ends(
                old,
                old_mounts,
                new,
                &mut new_mounts,
                new_children,
                prefix,
                old_suffix_start,
                new_suffix_start,
            );
        }

        match (prefix + suffix == old.len(), prefix + suffix == new.len()) {
            // Children remain only between the shared ends on the new side: insert.
            (true, false) => {
                let inserted = &new[prefix..new_suffix_start];
                // Anchor the inserted run against the shared end beside it: `First`/before the
                // suffix's first node when there is a suffix, otherwise `Last`/after the prefix's
                // last node. The prefix is shared (same index on both sides); the suffix starts at
                // different indices in old vs new.
                let (edge, new_boundary, old_boundary) = if suffix > 0 {
                    (ElementEdge::First, new_suffix_start, old_suffix_start)
                } else {
                    (ElementEdge::Last, prefix - 1, prefix - 1)
                };
                // A kept-template boundary's NEW mount is already committed, so anchor on the new
                // side; otherwise the boundary still shows its OLD committed mount (not yet diffed),
                // so anchor there before it moves.
                let (anchor_node, anchor_mount) = if pure_insert_anchor_keeps_template {
                    (
                        &new[new_boundary],
                        new_mounts[new_boundary].expect("shared boundary mount"),
                    )
                } else {
                    (&old[old_boundary], old_mounts[old_boundary])
                };
                self.create_and_insert(
                    edge,
                    inserted,
                    anchor_node,
                    anchor_mount,
                    parent,
                    |dom, offset, mount| {
                        let idx = prefix + offset;
                        new_mounts[idx] = Some(mount);
                        dom.set_mounted_fragment_child(new_children, idx, mount);
                    },
                );
                if !pure_insert_anchor_keeps_template {
                    // The insert anchored against the old mounts above, so the shared ends are
                    // diffed only now that the new run is placed.
                    self.diff_shared_ends(
                        old,
                        old_mounts,
                        new,
                        &mut new_mounts,
                        new_children,
                        prefix,
                        old_suffix_start,
                        new_suffix_start,
                    );
                }
            }
            // Children remain only between the shared ends on the old side: remove.
            (false, true) => {
                self.dom.remove_nodes(
                    self.to.as_deref_mut(),
                    &old[prefix..old_suffix_start],
                    &old_mounts[prefix..old_suffix_start],
                );
            }
            // Both ends are fully shared: nothing remains between them.
            (true, true) => {}
            // A genuine reorder remains between the shared ends: build the key map
            // and run the LIS over the reduced middle only.
            (false, false) => self.diff_keyed_middle(
                &old[prefix..old_suffix_start],
                &old_mounts[prefix..old_suffix_start],
                &new[prefix..new_suffix_start],
                parent,
                &mut new_mounts[prefix..new_suffix_start],
                new_children,
                prefix,
            ),
        }
    }

    /// Reconcile a genuine reorder confined between the shared ends.
    ///
    /// Builds the old-key map and runs the longest-increasing-subsequence search
    /// over this reduced middle only - the shared prefix/suffix were already
    /// diffed in place by [`Self::diff_keyed_children`]. `new_mounts` is the
    /// middle's slice of the parent's mount list and is filled with one mount per
    /// new middle child.
    ///
    /// Invariant: both `old` and `new` are non-empty (pure inserts and removes are
    /// handled by the caller) and share no prefix or suffix key.
    fn diff_keyed_middle(
        &mut self,
        old: &[VNode],
        old_mounts: &[MountId],
        new: &[VNode],
        parent: Option<MountRef>,
        new_mounts: &mut [Option<MountId>],
        new_children: FragmentMountWriter,
        new_offset: usize,
    ) {
        let old_key_to_old_index = old
            .iter()
            .enumerate()
            .map(|(i, o)| (o.key.as_ref().unwrap().as_str(), i))
            .collect::<FxHashMap<_, _>>();

        let mut old_is_shared = vec![false; old.len()];
        let mut shared_count = 0usize;
        let new_index_to_old_index = new
            .iter()
            .map(|node| {
                let key = node.key.as_ref().unwrap();
                if let Some(&index) = old_key_to_old_index.get(key.as_str()) {
                    if !old_is_shared[index] {
                        old_is_shared[index] = true;
                        shared_count += 1;
                    }
                    index
                } else {
                    usize::MAX
                }
            })
            .collect::<Box<[_]>>();

        if shared_count == 0 {
            self.create_and_insert(
                ElementEdge::First,
                new,
                old.first().unwrap(),
                old_mounts[0],
                parent,
                |dom, offset, mount| {
                    new_mounts[offset] = Some(mount);
                    dom.set_mounted_fragment_child(new_children, new_offset + offset, mount);
                },
            );
            self.dom
                .remove_nodes(self.to.as_deref_mut(), old, old_mounts);
            return;
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

        // Every shared child not in the stable LIS will move, so its committed
        // position is stale for the rest of this reorder. Mark them all up front
        // (O(1) each) so placement scans never anchor on a node that is mid-move,
        // and clear the markers once the fragment commits below. The set lives on
        // the runtime, so nothing is threaded or cloned per splice.
        let mut marked: Vec<MountId> = Vec::new();
        if self.has_writer() {
            let mut in_lis = vec![false; new.len()];
            for &idx in &lis_sequence {
                in_lis[idx] = true;
            }
            for (new_idx, &old_index) in new_index_to_old_index.iter().enumerate() {
                if old_index != usize::MAX && !in_lis[new_idx] {
                    let mount = old_mounts[old_index];
                    self.dom.runtime.mark_placement_stale(mount);
                    marked.push(mount);
                }
            }
        }

        // Diff the stable LIS children in place, in document order. A child whose
        // template changed gets a fresh mount, so its old mount is stale too.
        for idx in lis_sequence.iter().rev() {
            let old_index = new_index_to_old_index[*idx];
            let old_node = &old[old_index];
            let old_mount = old_mounts[old_index];
            let mount = DiffFrame::new(old_mount, old_node, &new[*idx]).diff_into(self);
            if mount != old_mount && self.has_writer() {
                self.dom.runtime.mark_placement_stale(old_mount);
                marked.push(old_mount);
            }
            new_mounts[*idx] = Some(mount);
            self.dom
                .set_mounted_fragment_child(new_children, new_offset + *idx, mount);
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
                &mut *new_mounts,
                new_children,
                new_offset,
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
                    new_mounts,
                    new_children,
                    new_offset,
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
                &mut *new_mounts,
                new_children,
                new_offset,
            );
        }

        // Remove the keyed children whose keys disappeared. They stayed mounted
        // until now so splices could still see them while choosing placement.
        for (_, (child_to_remove, mount_to_remove)) in old
            .iter()
            .zip(old_mounts)
            .enumerate()
            .filter(|(old_index, _)| !old_is_shared[*old_index])
        {
            child_to_remove.remove_node(*mount_to_remove, self.dom, self.to.as_deref_mut());
        }

        // The fragment is committed and the moved mounts now hold their new
        // positions, so clear their stale markers.
        for mount in marked {
            self.dom.runtime.unmark_placement_stale(mount);
        }
    }

    fn diff_child_pairs(
        &mut self,
        old: &[VNode],
        old_mounts: &[MountId],
        new: &[VNode],
        new_mounts: &mut [Option<MountId>],
        new_children: FragmentMountWriter,
        new_offset: usize,
    ) {
        let len = old.len().min(new.len());
        // Parent fragment mount lists are committed after the whole fragment
        // diff finishes. Diff pairs in document order so replacements at an
        // earlier index can still use later old siblings as live placement
        // anchors.
        for idx in 0..len {
            let old = &old[idx];
            let new = &new[idx];
            let mount = DiffFrame::new(old_mounts[idx], old, new).diff_into(self);
            new_mounts[new_offset + idx] = Some(mount);
            self.dom
                .set_mounted_fragment_child(new_children, new_offset + idx, mount);
        }
    }

    /// Create, move, or diff one non-LIS keyed splice range around an already materialized sibling.
    ///
    /// Invariant: every moved/replaced mount in this reorder is already marked stale on the runtime
    /// before this runs, so placement scans skip them without a threaded list.
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
        new_children: FragmentMountWriter,
        new_offset: usize,
    ) {
        let context = self.context();
        let sibling_mount = new_mounts[sibling_idx].expect("sibling");
        // The splice sits immediately next to its LIS-boundary sibling, which is
        // stable and never moves, so when that sibling has a live DOM edge it is
        // the exact anchor - O(1). Only when it has no live edge (e.g. an empty
        // fragment) do we walk the new sibling order, then the committed view;
        // both consult the runtime's stale set so they never anchor mid-move.
        let site = self.has_writer().then(|| {
            vnode_edge_site(
                edge,
                crate::MountedVNode::new(&new[sibling_idx], sibling_mount),
                self.dom,
            )
            .or_else(|| insertion_site_in_new_order(edge, new, new_mounts, sibling_idx, self.dom))
            .unwrap_or_else(|| {
                insertion_site_at(
                    edge,
                    crate::MountedVNode::new(&new[sibling_idx], sibling_mount),
                    self.dom,
                    context,
                )
            })
        });
        let runtime = self.dom.runtime.clone();
        let dom = &mut *self.dom;
        let to = self.to.as_deref_mut();
        let mut replaced_nodes = Vec::new();
        if let Some(site) = site {
            let to = to.expect("writer checked");
            at_site(site, to, runtime, |to| {
                let mut state = DiffState::new_with_context(dom, Some(to), context);
                state.create_or_diff_range(
                    new,
                    old,
                    old_mounts,
                    parent,
                    new_index_to_old_index,
                    range,
                    new_mounts,
                    new_children,
                    new_offset,
                    &mut replaced_nodes,
                )
            });
        } else {
            let mut state = DiffState::new_with_context(dom, to, context);
            state.create_or_diff_range(
                new,
                old,
                old_mounts,
                parent,
                new_index_to_old_index,
                range,
                new_mounts,
                new_children,
                new_offset,
                &mut replaced_nodes,
            );
        }
        for (node, mount) in replaced_nodes.into_iter().rev() {
            node.remove_node(mount, self.dom, self.to.as_deref_mut());
        }
    }

    /// Materialize every child in a keyed splice range at the current renderer insertion site.
    ///
    /// Invariant: `old_index != usize::MAX` means the index came from the old key map and is valid
    /// for `old`/`old_mounts`; `usize::MAX` means this new child has no previous mount.
    fn create_or_diff_range<'a>(
        &mut self,
        new: &[VNode],
        old: &'a [VNode],
        old_mounts: &[MountId],
        parent: Option<MountRef>,
        new_index_to_old_index: &[usize],
        range: std::ops::Range<usize>,
        new_mounts: &mut [Option<MountId>],
        new_children: FragmentMountWriter,
        new_offset: usize,
        replaced_nodes: &mut Vec<(&'a VNode, MountId)>,
    ) -> usize {
        let range_start = range.start;
        let mut nodes = 0;
        for (idx, new_node) in new[range.clone()].iter().enumerate() {
            let new_index = range_start + idx;
            let old_index = new_index_to_old_index[range_start + idx];
            let (created_nodes, mount) = if old_index != usize::MAX {
                let old_mount = old_mounts[old_index];
                let old_node = &old[old_index];
                let (nodes, mount) = if old_node.template != new_node.template {
                    let created = new_node.create_with_parents(
                        self.dom,
                        parent,
                        parent,
                        self.to.as_deref_mut(),
                    );
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
                // `old_mount` was already marked stale up front in
                // `diff_keyed_middle`, so placement scans skip it.
                (nodes, mount)
            } else {
                let created =
                    new_node.create_with_parents(self.dom, parent, parent, self.to.as_deref_mut());
                (created.nodes, created.mount)
            };
            new_mounts[new_index] = Some(mount);
            self.dom
                .set_mounted_fragment_child(new_children, new_offset + new_index, mount);
            nodes += created_nodes;
        }
        nodes
    }

    /// Create new non-keyed tail children next to a mounted sibling.
    ///
    /// Invariant: `sibling_mount` is still live and belongs to `sibling` when placement is chosen.
    fn create_and_insert(
        &mut self,
        edge: ElementEdge,
        new: &[VNode],
        sibling: &VNode,
        sibling_mount: MountId,
        parent: Option<MountRef>,
        created_mount: impl FnMut(&mut VirtualDom, usize, MountId),
    ) -> usize {
        self.create_children_at_site_with_mounts(
            new,
            parent,
            |state| {
                insertion_site_at(
                    edge,
                    crate::MountedVNode::new(sibling, sibling_mount),
                    state.dom,
                    state.context(),
                )
            },
            created_mount,
        )
    }

    /// Create `new` under `parent`. When a writer is active the children are placed at `site`
    /// (computed lazily - no-writer diffs only materialize mount state and resolve no placement);
    /// otherwise only their mount state is created.
    pub(super) fn create_children_at_site(
        &mut self,
        new: &[VNode],
        parent: Option<MountRef>,
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
        parent: Option<MountRef>,
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

/// Prefer anchors already materialized in the new sibling order.
///
/// Invariant: every `Some` entry in `new_mounts` owns a materialized sibling in new order. Pending
/// new siblings are `None` and therefore cannot be used as anchors.
fn insertion_site_in_new_order(
    edge: ElementEdge,
    new: &[VNode],
    new_mounts: &[Option<MountId>],
    sibling_idx: usize,
    dom: &VirtualDom,
) -> Option<InsertionSite> {
    match edge {
        ElementEdge::First => (sibling_idx..new.len()).find_map(|index| {
            let mount = new_mounts[index]?;
            new[index]
                .find_first_element(mount, dom)
                .map(InsertionSite::before)
        }),
        ElementEdge::Last => (0..=sibling_idx).rev().find_map(|index| {
            let mount = new_mounts[index]?;
            new[index]
                .find_last_element(mount, dom)
                .map(InsertionSite::after)
        }),
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
        match &self.dynamic_nodes[idx] {
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
