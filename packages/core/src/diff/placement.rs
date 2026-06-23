//! Renderer insertion-site selection for diff-created nodes.
//!
//! Invariants maintained here:
//! - Placement scans use the committed mount table by default. During an active same-template vnode
//!   diff, `DiffContext` supplies the old vnode for the current mount or parent mount because the
//!   committed mount table is not updated until the frame commits.
//! - `None` context does not mean there is no old vnode; it means no active diff-local vnode frame is
//!   available.
//! - If a mounted child has a render parent, that parent mount must still be live.
//! - Exact fragment-child access is used for diff internals; a shorter child-mount list is a mount
//!   table corruption bug.

use std::rc::Rc;

use crate::{
    MountedVNode, Runtime, VNode, VirtualDom, WriteMutations,
    arena::ElementId,
    innerlude::MountId,
    mutations::TargetedLazyScope,
    nodes::DynamicNode,
};

use super::{
    CreatedVNode,
    context::DiffContext,
    node::EdgeScan,
    template::{DynamicAnchor, DynamicNodeSlot},
};

#[derive(Clone, Copy)]
pub(super) enum ElementEdge {
    First,
    Last,
}

impl ElementEdge {
    pub(super) fn find_map<T>(
        self,
        len: usize,
        mut find: impl FnMut(usize) -> Option<T>,
    ) -> Option<T> {
        match self {
            ElementEdge::First => (0..len).find_map(&mut find),
            ElementEdge::Last => (0..len).rev().find_map(&mut find),
        }
    }
}

/// A renderer-level insertion site for nodes already on the renderer stack.
///
/// For `Before`/`After` the anchor is a sibling; for `AppendTo` it is the parent the nodes become
/// the last children of.
#[derive(Clone, Copy)]
pub(crate) enum InsertionSite {
    /// Insert immediately before the sibling anchor.
    Before(ElementId),
    /// Insert immediately after the sibling anchor.
    After(ElementId),
    /// Append as the last children of the parent anchor.
    AppendTo(ElementId),
}

impl InsertionSite {
    pub(super) fn before(anchor: ElementId) -> Self {
        Self::Before(anchor)
    }

    pub(super) fn after(anchor: ElementId) -> Self {
        Self::After(anchor)
    }

    pub(super) fn append_to(anchor: ElementId) -> Self {
        Self::AppendTo(anchor)
    }

    fn anchor(self) -> ElementId {
        match self {
            Self::Before(anchor) | Self::After(anchor) | Self::AppendTo(anchor) => anchor,
        }
    }

    fn create_and_place(
        self,
        to: &mut dyn WriteMutations,
        runtime: Rc<Runtime>,
        create: impl FnOnce(&mut dyn WriteMutations) -> usize,
    ) -> usize {
        self.create_and_place_with_result(to, runtime, |to| {
            let count = create(to);
            (count, count)
        })
    }

    fn create_and_place_with_result<R>(
        self,
        to: &mut dyn WriteMutations,
        runtime: Rc<Runtime>,
        create: impl FnOnce(&mut dyn WriteMutations) -> (usize, R),
    ) -> R {
        let anchor = self.anchor();
        let mut to = TargetedLazyScope::new(to, runtime, move |to| to.push_id(anchor));
        let (count, result) = create(&mut to);
        self.splice_stack(&mut to, count);
        result
    }

    fn splice_stack(self, to: &mut (impl WriteMutations + ?Sized), count: usize) {
        if count == 0 {
            return;
        }

        match self {
            Self::Before(_) => to.insert_before(count),
            Self::After(_) => to.insert_after(count),
            Self::AppendTo(_) => to.append_children(count),
        }
    }
}

/// Find an insertion site at the given edge of `vnode`'s live DOM: before its
/// first element, or after its last. If the vnode has no live DOM, walk mounted
/// parent slots until a live insertion point is found.
pub(super) fn insertion_site_at(
    edge: ElementEdge,
    vnode: MountedVNode<'_>,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> InsertionSite {
    let at_edge = vnode_edge_site(edge, vnode, dom);
    at_edge.unwrap_or_else(|| insertion_site_for_mounted_child(edge, vnode.mount(), dom, context))
}

/// Resolve the insertion site for a dynamic node slot inside `parent_mount`.
///
/// Invariant: adjacent dynamic slots at the same template insertion position keep their relative
/// order even when the position has no static anchor of its own.
pub(super) fn insertion_site_for_slot(
    parent_mount: MountId,
    slot: DynamicNodeSlot<'_>,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> InsertionSite {
    // An anchor can cover several adjacent dynamic nodes (`{a}{b}` lower to one anchor), so first
    // prefer the closest following sibling that shares it - this applies to both root-level and
    // nested slots - anchoring before its first live element. During a same-template parent diff,
    // following siblings have not been diffed yet, so their mounted slots still match the old view.
    if let Some(id) = with_parent_vnode(
        dom,
        parent_mount,
        context,
        ParentVersion::Old,
        |parent_vnode| {
            adjacent_dynamic_sibling_after_in_vnode(parent_vnode, parent_mount, slot, dom)
        },
    ) {
        return InsertionSite::before(id);
    }

    // If this is the last live dynamic node at the insertion position, place it after the closest
    // previous sibling before falling back to the static anchor or parent position. Previous
    // siblings have already been diffed, so in a same-template parent diff their mounted slots match
    // the new view, not the old view.
    if let Some(id) = with_parent_vnode(
        dom,
        parent_mount,
        context,
        ParentVersion::New,
        |parent_vnode| {
            adjacent_dynamic_sibling_before_in_vnode(parent_vnode, parent_mount, slot, dom)
        },
    ) {
        return InsertionSite::after(id);
    }

    if let Some(id) = dom.mounted_anchor_node(parent_mount, slot.anchor().anchor_index()) {
        return insertion_site_for_anchor_id(slot.anchor(), id.element_id());
    }

    insertion_site_for_mounted_child(ElementEdge::First, parent_mount, dom, context)
}

fn insertion_site_for_anchor_id(anchor: DynamicAnchor<'_>, anchor_id: ElementId) -> InsertionSite {
    if anchor.is_parent_append_target() {
        InsertionSite::append_to(anchor_id)
    } else if anchor.is_last_static_node() {
        InsertionSite::after(anchor_id)
    } else {
        InsertionSite::before(anchor_id)
    }
}

pub(super) fn create_at_site_with_mounts(
    content: &[VNode],
    parent: Option<MountId>,
    site: InsertionSite,
    dom: &mut VirtualDom,
    to: &mut dyn WriteMutations,
    mut created_mount: impl FnMut(&mut VirtualDom, usize, MountId),
) -> usize {
    at_site(site, to, dom.runtime.clone(), |to| {
        dom.create_children_with_mounts(Some(to), content, parent, parent, |dom, idx, mount| {
            created_mount(dom, idx, mount);
        })
    })
}

pub(super) fn create_at_site(
    content: &VNode,
    parent: Option<MountId>,
    site: InsertionSite,
    dom: &mut VirtualDom,
    to: &mut dyn WriteMutations,
) -> CreatedVNode {
    at_site_with_result(site, to, dom.runtime.clone(), |to| {
        let created = content.create_with_parents(dom, parent, parent, Some(to));
        (created.nodes, created)
    })
}

pub(super) fn at_site(
    site: InsertionSite,
    to: &mut dyn WriteMutations,
    runtime: Rc<Runtime>,
    create: impl FnOnce(&mut dyn WriteMutations) -> usize,
) -> usize {
    site.create_and_place(to, runtime, create)
}

fn at_site_with_result<R>(
    site: InsertionSite,
    to: &mut dyn WriteMutations,
    runtime: Rc<Runtime>,
    create: impl FnOnce(&mut dyn WriteMutations) -> (usize, R),
) -> R {
    site.create_and_place_with_result(to, runtime, create)
}

/// How streamed nodes attach to the renderer relative to an on-screen anchor element.
pub(crate) enum StreamPlacement {
    /// Replace the anchor element with the streamed nodes (or remove it when there are none).
    Replace(ElementId),
    /// Splice the streamed nodes in at the anchor's mounted insertion site.
    Insert(InsertionSite),
}

impl StreamPlacement {
    /// The placement to use when a resolved boundary's fallback has no DOM element to replace:
    /// anchor before `vnode`'s first live element, walking up to a live parent slot if it is empty.
    pub(crate) fn for_empty_fallback(vnode: MountedVNode<'_>, dom: &VirtualDom) -> Self {
        Self::Insert(insertion_site_at(ElementEdge::First, vnode, dom, None))
    }
}

/// Splice streamed nodes onto the renderer stack relative to an on-screen anchor, driving the
/// renderer through a *concrete* writer.
///
/// This is the concrete-writer counterpart to the diff's [`at_site`]: streaming suspense resume
/// can't reuse `at_site` because that wraps the writer in the `dyn`, portal-target-gated
/// [`TargetedLazyScope`], whereas the resume closure needs the concrete renderer to push
/// server-streamed DOM nodes that don't have an [`ElementId`] yet. `push_nodes` stacks those nodes
/// above the anchor and returns their count.
pub(crate) fn splice_streamed_nodes<M: WriteMutations>(
    to: &mut M,
    placement: StreamPlacement,
    push_nodes: impl FnOnce(&mut M) -> usize,
) -> usize {
    let anchor = match placement {
        StreamPlacement::Replace(id) => id,
        StreamPlacement::Insert(site) => site.anchor(),
    };
    to.push_id(anchor);
    let count = push_nodes(to);
    match placement {
        // `replace_with` consumes the anchor it replaces (and `remove` consumes it directly).
        StreamPlacement::Replace(_) => {
            if count > 0 {
                to.replace_with(count);
            } else {
                to.remove();
            }
        }
        // `insert_before`/`insert_after`/`append_children` leave the anchor on the stack, so pop it
        // to keep the renderer stack balanced - the discipline `TargetedLazyScope` applies on drop.
        StreamPlacement::Insert(site) => {
            site.splice_stack(to, count);
            to.pop();
        }
    }
    count
}

fn insertion_site_for_mounted_child(
    edge: ElementEdge,
    mut mount: MountId,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> InsertionSite {
    while let Some(parent_mount) = dom.mounted_render_parent(mount) {
        if let Some(site) =
            insertion_site_for_child_in_parent(edge, mount, parent_mount, dom, context)
        {
            return site;
        }
        mount = parent_mount;
    }
    InsertionSite::append_to(ElementId::ROOT)
}

/// Resolve a child mount's site inside a specific committed parent.
///
/// Invariant: if this returns `Some`, `mount` is owned by the returned parent slot. If no slot owns
/// `mount`, the caller must continue walking render parents.
fn insertion_site_for_child_in_parent(
    edge: ElementEdge,
    mount: MountId,
    parent_mount: MountId,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Option<InsertionSite> {
    // Child ownership is a committed-mount-table query. During a parent diff,
    // the new vnode may already have a different fragment shape, but the
    // fragment child mount list is not replaced until that parent diff
    // commits.
    with_parent_vnode(
        dom,
        parent_mount,
        context,
        ParentVersion::Old,
        |parent_vnode| {
            for slot in parent_vnode.dynamic_node_slots() {
                let idx = slot.index();
                match &parent_vnode.dynamic_node_values()[idx] {
                    DynamicNode::Fragment(children) => {
                        let site = dom.try_with_mounted_fragment_children(
                            parent_mount,
                            idx,
                            children.len(),
                            |child_mounts| {
                                let position =
                                    child_mounts.iter().position(|child| *child == mount)?;
                                insertion_site_near_fragment_child(
                                    edge,
                                    children,
                                    child_mounts,
                                    position,
                                    mount,
                                    dom,
                                )
                                .or_else(|| {
                                    Some(insertion_site_for_slot(parent_mount, slot, dom, context))
                                })
                            },
                        );
                        if let Some(site) = site.flatten() {
                            return Some(site);
                        }
                    }
                    DynamicNode::Component(_)
                        if dom.mounted_dynamic_component_root_mount(parent_mount, idx)
                            == Some(mount) =>
                    {
                        return Some(insertion_site_for_slot(parent_mount, slot, dom, context));
                    }
                    DynamicNode::Component(_) | DynamicNode::Text(_) => {}
                }
            }
            None
        },
    )
}

fn insertion_site_near_fragment_child(
    edge: ElementEdge,
    children: &[VNode],
    child_mounts: &[MountId],
    position: usize,
    mount: MountId,
    dom: &VirtualDom,
) -> Option<InsertionSite> {
    let following = || {
        children
            .iter()
            .zip(child_mounts)
            .skip(position + 1)
            .filter(|(_, m)| **m != mount)
            .find_map(|(child, &m)| child.find_first_element(m, dom).map(InsertionSite::before))
    };

    let previous = || {
        children
            .iter()
            .zip(child_mounts)
            .take(position)
            .rev()
            .filter(|(_, m)| **m != mount)
            .find_map(|(child, &m)| child.find_last_element(m, dom).map(InsertionSite::after))
    };

    match edge {
        ElementEdge::First => following().or_else(previous),
        ElementEdge::Last => previous().or_else(following),
    }
}

/// Find the next live dynamic sibling sharing the active slot's insertion position.
///
/// Invariant: `slot.index()` must exist in the committed parent view. Parent diffs only call this
/// for same-template vnode updates, so a missing slot is a template/diff-context bug.
fn adjacent_dynamic_sibling_after_in_vnode(
    parent_vnode: &VNode,
    parent_mount: MountId,
    slot: DynamicNodeSlot<'_>,
    dom: &VirtualDom,
) -> Option<ElementId> {
    parent_vnode
        .dynamic_node_slots_after_sharing_insertion_position(slot)
        .find_map(|sibling| {
            parent_vnode.dynamic_node_edge_element(
                parent_mount,
                sibling.index(),
                dom,
                EdgeScan::placement(dom),
                ElementEdge::First,
            )
        })
}

/// Find the previous live dynamic sibling sharing the active slot's insertion position.
fn adjacent_dynamic_sibling_before_in_vnode(
    parent_vnode: &VNode,
    parent_mount: MountId,
    slot: DynamicNodeSlot<'_>,
    dom: &VirtualDom,
) -> Option<ElementId> {
    let mut seen_slot = false;
    for sibling in parent_vnode.dynamic_node_slots().rev() {
        if !seen_slot {
            seen_slot = sibling.index() == slot.index();
            continue;
        }

        if !sibling.has_same_insertion_parent(slot) {
            continue;
        }

        if !sibling.shares_insertion_position(slot) {
            break;
        }

        if let Some(id) = parent_vnode.dynamic_node_edge_element(
            parent_mount,
            sibling.index(),
            dom,
            EdgeScan::placement(dom),
            ElementEdge::Last,
        ) {
            return Some(id);
        }
    }
    None
}

#[derive(Clone, Copy)]
enum ParentVersion {
    Old,
    New,
}

fn with_parent_vnode<T>(
    dom: &VirtualDom,
    parent_mount: MountId,
    context: Option<DiffContext<'_>>,
    version: ParentVersion,
    f: impl FnOnce(&VNode) -> Option<T>,
) -> Option<T> {
    if let Some(frame) = context.and_then(|context| context.for_mount(parent_mount)) {
        let vnode = match version {
            ParentVersion::Old => frame.old,
            ParentVersion::New => frame.new,
        };
        f(vnode)
    } else {
        let vnode = dom
            .current_mounted_view(parent_mount)
            .expect("parent mount");
        f(&vnode)
    }
}

/// The insertion site at one edge of a mounted vnode: anchored before its first live element
/// (`First`) or after its last (`Last`). `None` when the vnode contributes no live element.
fn vnode_edge_site(
    edge: ElementEdge,
    vnode: MountedVNode<'_>,
    dom: &VirtualDom,
) -> Option<InsertionSite> {
    match edge {
        ElementEdge::First => vnode.find_first_element(dom).map(InsertionSite::before),
        ElementEdge::Last => vnode.find_last_element(dom).map(InsertionSite::after),
    }
}
