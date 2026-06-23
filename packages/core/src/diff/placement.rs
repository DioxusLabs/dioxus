//! Renderer insertion-site selection for diff-created nodes.
//!
//! Invariants maintained here:
//! - Placement scans use the committed mount table by default. During an active same-template vnode
//!   diff, `DiffContext` supplies the old vnode for the current mount or parent mount because the
//!   committed mount table is not updated until the frame commits.
//! - `None` context does not mean there is no old vnode; it means no active diff-local vnode frame is
//!   available.
//! - Mounts marked placement-stale on the runtime are still present in committed fragment storage but
//!   must not be used as insertion anchors while a reorder or replacement is in progress.
//! - If a mounted child has a render parent, that parent mount must still be live.
//! - Exact fragment-child access is used for diff internals; a shorter child-mount list is a mount
//!   table corruption bug.

use std::rc::Rc;

use crate::{
    MountedVNode, Runtime, VNode, VirtualDom, WriteMutations,
    arena::ElementId,
    innerlude::{MountId, MountRef},
    mutations::{TargetedLazyScope, append_children_to},
    nodes::DynamicNode,
};

use super::{
    context::DiffContext,
    template::{DynamicAnchor, DynamicNodeSlot},
};

#[derive(Clone, Copy)]
pub(super) enum ElementEdge {
    First,
    Last,
}

impl ElementEdge {
    fn site(self, id: ElementId) -> InsertionSite {
        match self {
            ElementEdge::First => InsertionSite::before(id),
            ElementEdge::Last => InsertionSite::after(id),
        }
    }

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

/// Which side of an [`InsertionSite`]'s anchor `m` already-stacked DOM nodes are spliced onto.
#[derive(Clone, Copy)]
pub(super) enum InsertionEdge {
    /// Insert immediately before the sibling anchor.
    Before,
    /// Insert immediately after the sibling anchor.
    After,
    /// Append as the last children of the parent anchor.
    Append,
}

/// A renderer-level insertion site for nodes already on the renderer stack: the anchor element and
/// which side of it to splice onto. For `Before`/`After` the anchor is a sibling; for `Append` it
/// is the parent the nodes become the last children of.
#[derive(Clone, Copy)]
pub(crate) struct InsertionSite {
    anchor: ElementId,
    edge: InsertionEdge,
}

impl InsertionSite {
    pub(super) fn before(anchor: ElementId) -> Self {
        Self {
            anchor,
            edge: InsertionEdge::Before,
        }
    }

    pub(super) fn after(anchor: ElementId) -> Self {
        Self {
            anchor,
            edge: InsertionEdge::After,
        }
    }

    pub(super) fn append_to(anchor: ElementId) -> Self {
        Self {
            anchor,
            edge: InsertionEdge::Append,
        }
    }

    fn create_and_place(
        &self,
        to: &mut dyn WriteMutations,
        runtime: Rc<Runtime>,
        create: impl FnOnce(&mut dyn WriteMutations) -> usize,
    ) -> usize {
        match self.edge {
            InsertionEdge::Before | InsertionEdge::After => {
                let id = self.anchor;
                let before = matches!(self.edge, InsertionEdge::Before);
                let mut to = TargetedLazyScope::new(to, runtime, move |to| to.push_id(id));
                let count = create(&mut to);
                if count > 0 {
                    if before {
                        to.insert_before(count);
                    } else {
                        to.insert_after(count);
                    }
                }
                count
            }
            InsertionEdge::Append => append_children_to(to, self.anchor, runtime, create),
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
    at_edge.unwrap_or_else(|| insertion_site_for_mounted_child(vnode.mount(), dom, context))
}

/// Resolve the insertion site for a dynamic node slot inside `parent_mount`.
///
/// Invariant: slots with a mounted static anchor are placed relative to that anchor; slots without
/// one are parent-level append targets.
pub(super) fn insertion_site_for_slot(
    parent_mount: MountId,
    slot: DynamicNodeSlot<'_>,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> InsertionSite {
    // An anchor can cover several adjacent dynamic nodes (`{a}{b}` lower to one anchor), so first
    // prefer the closest following sibling that shares it — this applies to both root-level and
    // nested slots — anchoring before its first live element.
    if let Some(id) = parent_views(dom, parent_mount, context).find_committed_map(|parent_vnode| {
        adjacent_dynamic_sibling_after_in_vnode(parent_vnode.vnode(), parent_mount, slot, dom)
    }) {
        return InsertionSite::before(id);
    }

    if let Some(id) = dom.mounted_anchor_node(parent_mount, slot.anchor().anchor_index()) {
        return insertion_site_for_anchor_id(slot.anchor(), id.element_id());
    }

    insertion_site_for_mounted_child(parent_mount, dom, context)
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
    parent: Option<MountRef>,
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

pub(super) fn at_site(
    site: InsertionSite,
    to: &mut dyn WriteMutations,
    runtime: Rc<Runtime>,
    create: impl FnOnce(&mut dyn WriteMutations) -> usize,
) -> usize {
    site.create_and_place(to, runtime, create)
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
        StreamPlacement::Insert(site) => site.anchor,
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
        // to keep the renderer stack balanced — the discipline `TargetedLazyScope` applies on drop.
        StreamPlacement::Insert(site) => {
            if count > 0 {
                match site.edge {
                    InsertionEdge::Before => to.insert_before(count),
                    InsertionEdge::After => to.insert_after(count),
                    InsertionEdge::Append => to.append_children(count),
                }
            }
            to.pop();
        }
    }
    count
}

pub(super) fn insertion_site_for_mounted_child(
    mount: MountId,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> InsertionSite {
    let Some(parent_ref) = dom.mounted_render_parent(mount) else {
        return InsertionSite::append_to(ElementId::ROOT);
    };
    let parent_mount = parent_ref.mount;

    if let Some(site) = insertion_site_for_child_in_parent(mount, parent_mount, dom, context) {
        return site;
    }

    insertion_site_for_mounted_child(parent_mount, dom, context)
}

/// Resolve a child mount's site inside a specific committed parent.
///
/// Invariant: if this returns `Some`, `mount` is owned by the returned parent slot. If no slot owns
/// `mount`, the caller must continue walking render parents.
fn insertion_site_for_child_in_parent(
    mount: MountId,
    parent_mount: MountId,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Option<InsertionSite> {
    let parent_views = parent_views(dom, parent_mount, context);
    // Child ownership is a committed-mount-table query. During a parent diff,
    // the new vnode may already have a different fragment shape, but the
    // fragment child mount list is not replaced until that parent diff
    // commits.
    parent_views.find_committed_map(|parent_vnode| {
        for slot in parent_vnode.vnode().dynamic_node_slots() {
            let idx = slot.index();
            match &parent_vnode.dynamic_nodes[idx] {
                DynamicNode::Fragment(children) => {
                    let site = dom.try_with_mounted_fragment_children(
                        parent_mount,
                        idx,
                        children.len(),
                        |child_mounts| {
                            let position = child_mounts.iter().position(|child| *child == mount)?;
                            if let Some(id) = first_live_sibling_after(
                                children,
                                child_mounts,
                                position,
                                mount,
                                dom,
                            ) {
                                return Some(InsertionSite::before(id));
                            }
                            Some(insertion_site_for_slot(parent_mount, slot, dom, context))
                        },
                    );
                    match site {
                        Some(Some(site)) => return Some(site),
                        Some(None) | None => continue,
                    }
                }
                DynamicNode::Component(_) => {
                    if dom.mounted_dynamic_component_root_mount(parent_mount, idx) == Some(mount) {
                        return Some(insertion_site_for_slot(parent_mount, slot, dom, context));
                    }
                }
                DynamicNode::Text(_) => {}
            }
        }
        None
    })
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
    for sibling in parent_vnode.dynamic_node_slots_after_sharing_insertion_position(slot) {
        if let Some(id) =
            live_dynamic_slot_first_element(parent_vnode, parent_mount, sibling.index(), dom)
        {
            return Some(id);
        }
    }
    None
}

/// Find a live DOM edge for one dynamic slot.
///
/// Invariant: component root mounts returned by the scope state own a committed vnode; fragment
/// slots have exactly one mount per child. The slot being inspected is outside the active fragment
/// reorder/replacement, so its mounts are not marked placement-stale for the current placement scan.
fn live_dynamic_slot_first_element(
    vnode: &VNode,
    mount: MountId,
    idx: usize,
    dom: &VirtualDom,
) -> Option<ElementId> {
    match &vnode.dynamic_nodes[idx] {
        DynamicNode::Text(_) => dom
            .mounted_dynamic_text_node(mount, idx)
            .map(|id| id.element_id()),
        DynamicNode::Fragment(children) => dom
            .try_with_mounted_fragment_children(mount, idx, children.len(), |child_mounts| {
                children
                    .iter()
                    .zip(child_mounts)
                    .find_map(|(child, mount)| {
                        vnode_edge_element(
                            MountedVNode::new(child, *mount),
                            dom,
                            ElementEdge::First,
                        )
                    })
            })
            .flatten(),
        DynamicNode::Component(_) => {
            let component_root_mount = dom.mounted_dynamic_component_root_mount(mount, idx)?;
            let vnode = dom.current_mounted_view(component_root_mount)?;
            vnode_edge_element(
                MountedVNode::new(&vnode, component_root_mount),
                dom,
                ElementEdge::First,
            )
        }
    }
}

fn parent_views<'a>(
    dom: &VirtualDom,
    parent_mount: MountId,
    context: Option<DiffContext<'a>>,
) -> ParentViews<'a> {
    if let Some(context) = context.and_then(|context| context.for_mount(parent_mount)) {
        return ParentViews::Context {
            mount: parent_mount,
            old: context.old,
        };
    }
    ParentViews::Mounted {
        mount: parent_mount,
        view: dom
            .current_mounted_view(parent_mount)
            .expect("parent mount"),
    }
}

enum ParentViews<'a> {
    Context { mount: MountId, old: &'a VNode },
    Mounted { mount: MountId, view: VNode },
}

impl<'a> ParentViews<'a> {
    fn find_committed_map<T>(&self, mut f: impl FnMut(MountedVNode<'_>) -> Option<T>) -> Option<T> {
        match self {
            Self::Context { mount, old } => f(MountedVNode::new(old, *mount)),
            Self::Mounted { mount, view } => f(MountedVNode::new(view, *mount)),
        }
    }
}

fn first_live_sibling_after(
    children: &[VNode],
    child_mounts: &[MountId],
    position: usize,
    mount: MountId,
    dom: &VirtualDom,
) -> Option<ElementId> {
    children
        .iter()
        .zip(child_mounts)
        .skip(position + 1)
        .find_map(|(child, m)| {
            let m = *m;
            // Skip the node itself and any sibling the active diff has already
            // moved/replaced — its committed position is stale.
            if m == mount || dom.runtime.is_placement_stale(m) {
                return None;
            }
            child.find_first_element(m, dom)
        })
}

fn vnode_edge_element(
    vnode: MountedVNode<'_>,
    dom: &VirtualDom,
    edge: ElementEdge,
) -> Option<ElementId> {
    match edge {
        ElementEdge::First => vnode.find_first_element(dom),
        ElementEdge::Last => vnode.find_last_element(dom),
    }
}

/// The insertion site at one edge of a mounted vnode: anchored before its first live element
/// (`First`) or after its last (`Last`). `None` when the vnode contributes no live element.
pub(super) fn vnode_edge_site(
    edge: ElementEdge,
    vnode: MountedVNode<'_>,
    dom: &VirtualDom,
) -> Option<InsertionSite> {
    vnode_edge_element(vnode, dom, edge).map(|id| edge.site(id))
}
