//! Renderer insertion-site selection for diff-created nodes.
//!
//! Invariants maintained here:
//! - Placement is chosen from the committed parent view unless a `DiffContext` identifies the
//!   active vnode currently being diffed.
//! - Mounts in the caller-provided skip list are still present in committed fragment storage but
//!   have already been claimed by an earlier replacement/splice; skip filtering happens while
//!   scanning that fragment's committed child-mount list.
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
    CreatedNodes,
    context::DiffContext,
    template::{
        DynamicNodeSlot, SlotPlacement, dynamic_node_slot, dynamic_node_slots_in_document_order,
    },
};

#[derive(Clone, Copy)]
pub(super) enum ElementEdge {
    First,
    Last,
}

impl ElementEdge {
    fn anchor(self, id: ElementId) -> DomAnchor {
        match self {
            ElementEdge::First => DomAnchor::Before(id),
            ElementEdge::Last => DomAnchor::After(id),
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

/// A renderer-level position where `m` DOM nodes, already on the renderer stack,
/// should be spliced in.
#[derive(Clone)]
pub(super) enum DomAnchor {
    Before(ElementId),
    After(ElementId),
}

/// A renderer-level insertion site for nodes already on the renderer stack.
#[derive(Clone)]
pub(super) enum InsertionSite {
    AtAnchor(DomAnchor),
    Slot {
        parent: ElementId,
        placement: SlotPlacement,
    },
    AppendTo(ElementId),
}

impl InsertionSite {
    fn create_and_place(
        &self,
        to: &mut dyn WriteMutations,
        runtime: Rc<Runtime>,
        create: impl FnOnce(&mut dyn WriteMutations) -> usize,
    ) -> usize {
        match self {
            InsertionSite::AtAnchor(anchor) => {
                let (id, before) = match anchor {
                    DomAnchor::Before(id) => (*id, true),
                    DomAnchor::After(id) => (*id, false),
                };
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
            InsertionSite::AppendTo(id) => append_children_to(to, *id, runtime, create),
            InsertionSite::Slot { parent, placement } => {
                insert_at_slot(to, *parent, placement.clone(), runtime, create)
            }
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
    let at_edge =
        vnode_edge_element(vnode, dom, edge).map(|id| InsertionSite::AtAnchor(edge.anchor(id)));
    at_edge.unwrap_or_else(|| insertion_site_for_mounted_child(vnode.mount(), dom, context))
}

/// Resolve the insertion site for a dynamic node slot inside `parent_mount`.
///
/// Invariant: root-level slots are placed relative to committed root siblings; non-root slots are
/// placed inside their enclosing static root, which must be mounted before renderer placement is
/// requested.
pub(super) fn insertion_site_for_slot(
    parent_mount: MountId,
    slot: DynamicNodeSlot<'_>,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> InsertionSite {
    let root_idx = slot.root_index();
    // Every node cursor entry the diff hands us starts with the root index
    // (see `compile_template` and rsx codegen), so the empty cursor is
    // unreachable in practice.
    if slot.is_root_level() {
        let our_root_idx = root_idx;
        // A root-level anchor can cover several adjacent dynamic values
        // (`{a}{b}` lowers to one append-at-root anchor). The closest following
        // sibling is the next live value sharing this anchor, so check it before
        // scanning later root positions or walking up to committed root siblings.
        if let Some(id) =
            parent_views(dom, parent_mount, context).find_committed_map(|parent_vnode| {
                adjacent_dynamic_sibling_after_in_vnode(
                    parent_vnode.vnode(),
                    parent_mount,
                    slot,
                    dom,
                )
            })
        {
            return InsertionSite::AtAnchor(DomAnchor::Before(id));
        }
        if let Some(id) = root_content_after_slot(parent_mount, our_root_idx, dom) {
            return InsertionSite::AtAnchor(DomAnchor::Before(id));
        }
        return insertion_site_for_mounted_child(parent_mount, dom, context);
    }

    // `cursor.len() > 1` means we're walking inside a template element. The
    // enclosing root is therefore part of the same mounted template and must
    // exist whenever renderer placement is requested.
    let enclosing = dom
        .mounted_root_node(parent_mount, root_idx)
        .expect("bad slot root");
    debug_assert!(
        dom.element_exists_for_mount(parent_mount, enclosing),
        "bad slot root"
    );
    if let Some(id) = parent_views(dom, parent_mount, context).find_committed_map(|parent_vnode| {
        adjacent_dynamic_sibling_after_in_vnode(parent_vnode.vnode(), parent_mount, slot, dom)
    }) {
        return InsertionSite::AtAnchor(DomAnchor::Before(id));
    }
    InsertionSite::Slot {
        parent: enclosing.element_id(),
        placement: slot.placement(),
    }
}

pub(super) fn create_at_site(
    content: &[VNode],
    parent: Option<MountRef>,
    site: InsertionSite,
    dom: &mut VirtualDom,
    to: &mut dyn WriteMutations,
) -> CreatedNodes {
    let mut mounts = Vec::new();
    let nodes = at_site(site, to, dom.runtime.clone(), |to| {
        let created = dom.create_children_with_parents(Some(to), content, parent, parent);
        mounts = created.mounts;
        created.nodes
    });
    CreatedNodes { nodes, mounts }
}

pub(super) fn at_site(
    site: InsertionSite,
    to: &mut dyn WriteMutations,
    runtime: Rc<Runtime>,
    create: impl FnOnce(&mut dyn WriteMutations) -> usize,
) -> usize {
    site.create_and_place(to, runtime, create)
}

fn insert_at_slot(
    to: &mut dyn WriteMutations,
    root_id: ElementId,
    placement: SlotPlacement,
    runtime: Rc<Runtime>,
    create: impl FnOnce(&mut dyn WriteMutations) -> usize,
) -> usize {
    let mut to = TargetedLazyScope::new(to, runtime, move |to| {
        to.push_id(root_id);
        for depth in 1..placement.parent_path.len() {
            to.child(placement.parent_path.segment(depth) as usize);
        }
        if !placement.appends {
            to.child(placement.static_insertion_index);
        }
    });
    let count = create(&mut to);
    if count > 0 {
        if placement.appends {
            to.append_children(count);
        } else {
            to.insert_before(count);
        }
    }
    count
}

fn insertion_site_for_mounted_child(
    mount: MountId,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> InsertionSite {
    let Some(parent_ref) = dom.mounted_render_parent(mount) else {
        return InsertionSite::AppendTo(ElementId::ROOT);
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
        for slot in dynamic_node_slots_in_document_order(parent_vnode.vnode()) {
            let idx = slot.index();
            match parent_vnode.dynamic_values[idx].node() {
                DynamicNode::Fragment(children) => {
                    let child_mounts =
                        dom.mounted_fragment_children_exact(parent_mount, idx, children.len());
                    let Some(position) = child_mounts.iter().position(|child| *child == mount)
                    else {
                        continue;
                    };
                    let sib = first_live_sibling_after(children, &child_mounts, position, mount, dom);
                    if std::env::var_os("FUZZ_DBG").is_some() {
                        eprintln!(
                            "[walk] FRAGMENT idx={idx} len={} mount={mount:?} pos={position} child_mounts={child_mounts:?} sibling_after={sib:?}",
                            children.len()
                        );
                    }
                    if let Some(id) = sib {
                        return Some(InsertionSite::AtAnchor(DomAnchor::Before(id)));
                    }
                    return Some(insertion_site_for_slot(parent_mount, slot, dom, context));
                }
                DynamicNode::Component(_) => {
                    if dom.unchecked_mounted_dynamic_component_root_mount(parent_mount, idx)
                        == mount
                    {
                        if std::env::var_os("FUZZ_DBG").is_some() {
                            eprintln!("[walk] COMPONENT idx={idx} root mount={mount:?}");
                        }
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
    let active_slot = dynamic_node_slot(parent_vnode, slot.index()).expect("bad active slot");
    first_live_slot_after(
        parent_vnode,
        parent_mount,
        active_slot.index(),
        dom,
        |sibling| {
            let shares_position = sibling.shares_insertion_position(active_slot);
            if sibling.parent_path() == active_slot.parent_path() && !shares_position {
                None
            } else {
                Some(shares_position)
            }
        },
    )
}

fn first_live_slot_after(
    vnode: &VNode,
    mount: MountId,
    after_idx: usize,
    dom: &VirtualDom,
    mut scan: impl FnMut(DynamicNodeSlot<'_>) -> Option<bool>,
) -> Option<ElementId> {
    let mut after_active_slot = false;
    for slot in dynamic_node_slots_in_document_order(vnode) {
        if !after_active_slot {
            if slot.index() == after_idx {
                after_active_slot = true;
            }
            continue;
        }
        match scan(slot) {
            Some(true) => {
                if let Some(id) = live_dynamic_slot_first_element(vnode, mount, slot.index(), dom) {
                    return Some(id);
                }
            }
            Some(false) => {}
            None => break,
        }
    }
    None
}

/// Find a live DOM edge for one dynamic slot.
///
/// Invariant: component root mounts returned by the scope state own a committed vnode; fragment
/// slots have exactly one mount per child. The slot being inspected is outside the active committed
/// fragment child list, so caller skip lists cannot contain the mounts it owns.
fn live_dynamic_slot_first_element(
    vnode: &VNode,
    mount: MountId,
    idx: usize,
    dom: &VirtualDom,
) -> Option<ElementId> {
    let target_id = dom.current_render_target_id();
    match vnode.dynamic_values[idx].node() {
        DynamicNode::Text(_) => dom
            .mounted_dynamic_text_node(mount, idx)
            .filter(|id| dom.element_exists_in_target(target_id, *id))
            .map(|id| id.element_id()),
        DynamicNode::Fragment(children) => {
            let child_mounts = dom.mounted_fragment_children_exact(mount, idx, children.len());
            children
                .iter()
                .zip(child_mounts)
                .find_map(|(child, mount)| {
                    vnode_edge_element(MountedVNode::new(child, mount), dom, ElementEdge::First)
                })
        }
        DynamicNode::Component(_) => {
            let component_root_mount =
                dom.unchecked_mounted_dynamic_component_root_mount(mount, idx);
            let vnode = dom
                .current_mounted_view(component_root_mount)
                .expect("component vnode");
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

fn root_content_after_slot(
    parent_mount: MountId,
    our_root_idx: usize,
    dom: &VirtualDom,
) -> Option<ElementId> {
    // Probe the committed mount view of `parent_mount`. The diff context's
    // `old` snapshot matches the committed view by construction (both are
    // the pre-diff `mount.node`), so reading directly from the mount
    // registry covers both cases. Callers (`insertion_site_for_slot` after the
    // path-length check) only reach here with a `parent_mount` that's
    // currently being diffed, so its mount is registered.
    let probe = dom
        .current_mounted_view(parent_mount)
        .expect("parent_root_after requires a live parent mount");

    // Values sharing this slot's anchor were already considered by the caller
    // (adjacent `{a}{b}` root dynamics lower to one anchor), so begin at the
    // next root position and look for the first later root's live content.
    ((our_root_idx + 1)..probe.template.root_count()).find_map(|next_cursor| {
        find_root_dynamic_slot(&probe, next_cursor, ElementEdge::First, |slot| {
            live_dynamic_slot_first_element(&probe, parent_mount, slot.index(), dom)
        })
        .or_else(|| static_root_element(&probe, parent_mount, next_cursor, dom))
    })
}

pub(super) fn find_root_dynamic_slot<T>(
    vnode: &VNode,
    cursor_idx: usize,
    edge: ElementEdge,
    mut find: impl FnMut(DynamicNodeSlot<'_>) -> Option<T>,
) -> Option<T> {
    let mut slots = dynamic_node_slots_in_document_order(vnode)
        .filter(|slot| slot.is_root_level() && slot.root_index() == cursor_idx);
    match edge {
        ElementEdge::First => slots.find_map(&mut find),
        ElementEdge::Last => {
            let mut found = None;
            for slot in slots {
                found = find(slot);
            }
            found
        }
    }
}

pub(super) fn static_root_element(
    vnode: &VNode,
    mount: MountId,
    root_idx: usize,
    dom: &VirtualDom,
) -> Option<ElementId> {
    debug_assert!(
        root_idx < vnode.template.root_count(),
        "root lookup must stay within the vnode template"
    );
    dom.mounted_root_node(mount, root_idx)
        .filter(|id| dom.element_exists_for_mount(mount, *id))
        .map(|id| id.element_id())
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
