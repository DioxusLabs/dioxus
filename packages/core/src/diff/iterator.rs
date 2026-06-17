//! Fragment child reconciliation.
//!
//! Invariants maintained here:
//! - Non-empty fragment diffs always return one mount per new child.
//! - Pairwise diffs run in document order so earlier replacements can still use later committed
//!   siblings as placement anchors.
//! - Keyed removals are delayed until every splice has selected its insertion site.
//! - `usize::MAX` in `new_index_to_old_index` is the only marker for a newly created keyed child;
//!   every other index must point into the old sibling list.

use crate::{
    DynamicNode, ElementId, VirtualDom,
    diff::{
        context::{DiffFrame, DiffState},
        placement::{
            DomAnchor, ElementEdge, InsertionSite, at_site, create_at_site, insertion_site_at,
        },
    },
    innerlude::{MountId, MountRef, WriteMutations},
    mutations::reborrow_writer,
    nodes::VNode,
};

use rustc_hash::{FxHashMap, FxHashSet};

impl DiffState<'_, '_, '_> {
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
    ) -> Vec<MountId> {
        use std::cmp::Ordering;

        // Handled these cases in `diff_children` before calling this function.
        dioxus_debug_assert!(!new.is_empty());
        dioxus_debug_assert!(!old.is_empty());

        let mut new_mounts = vec![None; new.len()];
        match old.len().cmp(&new.len()) {
            Ordering::Greater => {}
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
        if old.len() > new.len() {
            // Keep removed tail children mounted until paired replacements
            // have selected placement anchors. The committed parent fragment
            // mount list is not replaced until this fragment diff returns.
            self.dom.remove_nodes(
                reborrow_writer(&mut self.to),
                &old[new.len()..],
                &old_mounts[new.len()..],
            );
        }
        new_mounts
            .into_iter()
            .map(|mount| mount.expect("new child should have a mount after non-keyed diff"))
            .collect()
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
            let created = if self.to.is_some() {
                let site = insertion_site_at(
                    ElementEdge::First,
                    crate::MountedVNode::new(first_old, old_mounts[0]),
                    self.placement_skip(),
                    self.dom,
                    self.context(),
                );
                let to = reborrow_writer(&mut self.to)
                    .expect("writer presence checked before placement");
                create_at_site(new, parent, site, self.dom, to)
            } else {
                self.dom
                    .create_children_with_parents(None, new, parent, parent)
            };
            self.dom
                .remove_nodes(reborrow_writer(&mut self.to), old, old_mounts);
            return created.mounts;
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
        let mut mounted_new = Vec::new();
        let mut claimed_splice_mounts = Vec::new();
        // `lis_sequence` is kept in the order expected by the splice range
        // logic below. Diff the stable keyed children in document order so a
        // replacement does not remove a later sibling before an earlier child
        // needs it as a placement anchor.
        for idx in lis_sequence.iter().rev() {
            let old_index = new_index_to_old_index[*idx];
            let old_node = &old[old_index];
            let old_mount = old_mounts[old_index];
            let mount = DiffFrame::new(old_mount, old_node, &new[*idx]).diff_into(self);
            if mount != old_mount {
                // The committed parent child list still contains the old
                // mount until this keyed fragment commits. If diffing a
                // stable child replaced its mount, later splice placement
                // must not use the old position as an anchor.
                claimed_splice_mounts.push(old_mount);
            }
            new_mounts[*idx] = Some(mount);
            mounted_new.push(MountedSibling { index: *idx, mount });
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
                &mut mounted_new,
                &mut claimed_splice_mounts,
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
                    &mut mounted_new,
                    &mut claimed_splice_mounts,
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
                &mut mounted_new,
                &mut claimed_splice_mounts,
            );
        }

        // Keep removed keyed children mounted until every splice has chosen
        // its placement. The committed parent fragment mount list remains the
        // source of placement anchors until the fragment commits below.
        for (child_to_remove, mount_to_remove) in old
            .iter()
            .zip(old_mounts)
            .filter(|(child, _)| !shared_keys.contains(child.key.as_ref().unwrap()))
        {
            child_to_remove.remove_node(*mount_to_remove, self.dom, reborrow_writer(&mut self.to));
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
        // Parent fragment mount lists are committed after the whole fragment
        // diff finishes. Diff pairs in document order so replacements at an
        // earlier index can still use later old siblings as live placement
        // anchors.
        for idx in 0..len {
            let old = &old[idx];
            let new = &new[idx];
            let mount = DiffFrame::new(old_mounts[idx], old, new).diff_into(self);
            new_mounts[new_offset + idx] = Some(mount);
        }
    }

    /// Create, move, or diff one non-LIS keyed splice range around an already materialized sibling.
    ///
    /// Invariant: the outer insertion site is selected before any old mounts in `range` are
    /// removed. `claimed_splice_mounts` contains old mounts that are still visible in the committed
    /// parent list but must not be used as anchors.
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
        mounted_new: &mut Vec<MountedSibling>,
        claimed_splice_mounts: &mut Vec<MountId>,
    ) {
        let current_splice_mounts =
            collect_splice_mounts(old_mounts, new_index_to_old_index, range.clone());
        // Splice ranges are processed while the parent fragment still exposes
        // its committed old child list. Once an earlier splice has claimed an
        // old mount, later placement lookups must not use that old position as
        // an anchor; it either already moved or was replaced by the new range.
        let mut skip =
            Vec::with_capacity(claimed_splice_mounts.len() + current_splice_mounts.len());
        skip.extend(claimed_splice_mounts.iter().copied());
        skip.extend(current_splice_mounts.iter().copied());
        let inner_skip = claimed_splice_mounts.clone();
        let context = self.context();
        let sibling_mount = new_mounts[sibling_idx].expect("sibling should already be diffed");
        let site = self.to.is_some().then(|| {
            insertion_site_in_new_order(edge, new, mounted_new, sibling_idx, self.dom)
                .unwrap_or_else(|| {
                    insertion_site_at(
                        edge,
                        crate::MountedVNode::new(&new[sibling_idx], sibling_mount),
                        &skip,
                        self.dom,
                        context,
                    )
                })
        });
        let runtime = self.dom.runtime.clone();
        let dom = &mut *self.dom;
        let to = reborrow_writer(&mut self.to);
        let mut replaced_nodes = Vec::new();
        if let Some(site) = site {
            let to = to.expect("writer presence checked before splice placement");
            at_site(site, to, runtime, |to| {
                let mut state = DiffState::new_with_context_and_placement_skip(
                    dom,
                    Some(to),
                    context,
                    &inner_skip,
                );
                state.create_or_diff_range(
                    new,
                    old,
                    old_mounts,
                    parent,
                    new_index_to_old_index,
                    range,
                    new_mounts,
                    mounted_new,
                    &mut replaced_nodes,
                )
            });
        } else {
            let mut state =
                DiffState::new_with_context_and_placement_skip(dom, None, context, &inner_skip);
            state.create_or_diff_range(
                new,
                old,
                old_mounts,
                parent,
                new_index_to_old_index,
                range,
                new_mounts,
                mounted_new,
                &mut replaced_nodes,
            );
        }
        for (node, mount) in replaced_nodes.into_iter().rev() {
            node.remove_node(mount, self.dom, reborrow_writer(&mut self.to));
        }
        claimed_splice_mounts.extend(current_splice_mounts);
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
        mounted_new: &mut Vec<MountedSibling>,
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
                        reborrow_writer(&mut self.to),
                    );
                    replaced_nodes.push((old_node, old_mount));
                    (created.nodes, created.mount)
                } else {
                    let mount = DiffFrame::new(old_mount, old_node, new_node).diff_into(self);
                    let nodes = if let Some(to) = reborrow_writer(&mut self.to) {
                        crate::MountedVNode::new(new_node, mount).push_all_root_nodes(self.dom, to)
                    } else {
                        0
                    };
                    (nodes, mount)
                };
                self.push_placement_skip(old_mount);
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
            mounted_new.push(MountedSibling {
                index: new_index,
                mount,
            });
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
    ) -> crate::diff::CreatedNodes {
        if self.to.is_some() {
            let site = insertion_site_at(
                edge,
                crate::MountedVNode::new(sibling, sibling_mount),
                self.placement_skip(),
                self.dom,
                self.context(),
            );
            let to =
                reborrow_writer(&mut self.to).expect("writer presence checked before placement");
            create_at_site(new, parent, site, self.dom, to)
        } else {
            self.dom
                .create_children_with_parents(None, new, parent, parent)
        }
    }
}

/// Collect old mounts claimed by one keyed splice range.
///
/// Invariant: every non-`usize::MAX` entry in `new_index_to_old_index` is a valid old child index.
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
        .filter_map(|idx| {
            let old_index = new_index_to_old_index[idx];
            (old_index != usize::MAX).then(|| old_mounts[old_index])
        })
        .collect()
}

#[derive(Clone, Copy)]
struct MountedSibling {
    index: usize,
    mount: MountId,
}

/// Prefer anchors already materialized in the new sibling order.
///
/// Invariant: every entry in `mounted_new` owns a live mount in `new` order. Pending new siblings are
/// not representable in this slice and therefore cannot be used as anchors.
fn insertion_site_in_new_order(
    edge: ElementEdge,
    new: &[VNode],
    mounted_new: &[MountedSibling],
    sibling_idx: usize,
    dom: &VirtualDom,
) -> Option<InsertionSite> {
    match edge {
        ElementEdge::First => mounted_new
            .iter()
            .filter(|sibling| sibling.index >= sibling_idx)
            .filter_map(|sibling| {
                new[sibling.index]
                    .find_first_element(sibling.mount, dom)
                    .map(|id| (sibling.index, id))
            })
            .min_by_key(|(index, _)| *index)
            .map(|(_, id)| InsertionSite::AtAnchor(DomAnchor::Before(id))),
        ElementEdge::Last => mounted_new
            .iter()
            .filter(|sibling| sibling.index <= sibling_idx)
            .filter_map(|sibling| {
                new[sibling.index]
                    .find_last_element(sibling.mount, dom)
                    .map(|id| (sibling.index, id))
            })
            .max_by_key(|(index, _)| *index)
            .map(|(_, id)| InsertionSite::AtAnchor(DomAnchor::After(id))),
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
