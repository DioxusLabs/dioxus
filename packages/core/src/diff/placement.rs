//! Renderer insertion-site selection for diff-created nodes.
//!
//! Invariants maintained here:
//! - If a mounted child has a render parent, that parent mount must still be live.
//! - Exact fragment-child access is used for diff internals; a shorter child-mount list is a mount
//!   table corruption bug.

use std::rc::Rc;

use crate::{
    MountedVNode, Runtime, VNode, VirtualDom, WriteMutations, arena::ElementId, innerlude::MountId,
    mutations::TargetedLazyScope, nodes::DynamicNode,
};

use super::{
    CreatedVNode,
    context::DiffContext,
    node::EdgeScan,
    template::{DynamicAnchor, DynamicNodeSlot},
};

#[derive(Clone, Copy)]
pub(crate) enum ElementEdge {
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
    pub(crate) fn before(anchor: ElementId) -> Self {
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

/// Find an insertion site before `vnode`'s first live element. If the vnode has
/// no live DOM, walk mounted parent slots until a live insertion point is found.
pub(crate) fn insertion_site_at(
    vnode: MountedVNode<'_>,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> InsertionSite {
    PlacementResolver::new(dom, context).resolve_vnode_site(vnode)
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
    PlacementResolver::new(dom, context).resolve_slot(parent_mount, slot)
}

/// Resolves renderer insertion sites by trying local live-edge candidates before lifting an
/// empty mount to the dynamic slot that owns it.
struct PlacementResolver<'dom, 'ctx> {
    dom: &'dom VirtualDom,
    context: Option<DiffContext<'ctx>>,
}

impl<'dom, 'ctx> PlacementResolver<'dom, 'ctx> {
    fn new(dom: &'dom VirtualDom, context: Option<DiffContext<'ctx>>) -> Self {
        Self { dom, context }
    }

    fn resolve_vnode_site(&self, vnode: MountedVNode<'_>) -> InsertionSite {
        self.vnode_first_site(vnode)
            .unwrap_or_else(|| self.resolve_mount_site(vnode.mount()))
    }

    fn resolve_slot(&self, parent_mount: MountId, slot: DynamicNodeSlot<'_>) -> InsertionSite {
        // An anchor can cover several adjacent dynamic nodes (`{a}{b}` lower to one anchor), so
        // first prefer the closest following sibling that shares it - this applies to both
        // root-level and nested slots - anchoring before its first live element. During a
        // same-template parent diff, following siblings have not been diffed yet, so their mounted
        // slots still match the old view.
        if let Some(id) = self.with_parent_vnode(parent_mount, ParentVersion::Old, |parent_vnode| {
            self.adjacent_dynamic_sibling_after_in_vnode(parent_vnode, parent_mount, slot)
        }) {
            return InsertionSite::before(id);
        }

        // If this is the last live dynamic node at the insertion position, place it after the
        // closest previous sibling before falling back to the static anchor or parent position.
        // Previous siblings have already been diffed, so in a same-template parent diff their
        // mounted slots match the new view, not the old view.
        if let Some(id) = self.with_parent_vnode(parent_mount, ParentVersion::New, |parent_vnode| {
            self.adjacent_dynamic_sibling_before_in_vnode(parent_vnode, parent_mount, slot)
        }) {
            return InsertionSite::after(id);
        }

        if let Some(id) = self
            .dom
            .mounted_anchor_node(parent_mount, slot.anchor().anchor_index())
        {
            return self.anchor_site(slot.anchor(), id.element_id());
        }

        self.resolve_mount_site(parent_mount)
    }

    fn resolve_mount_site(&self, mut mount: MountId) -> InsertionSite {
        while let Some(parent_mount) = self.dom.mounted_render_parent(mount) {
            if let Some(site) = self.resolve_child_in_parent(mount, parent_mount) {
                return site;
            }
            mount = parent_mount;
        }
        InsertionSite::append_to(ElementId::ROOT)
    }

    /// Resolve a child mount's site inside a specific committed parent.
    ///
    /// Invariant: if this returns `Some`, `mount` is owned by the returned parent slot.
    fn resolve_child_in_parent(
        &self,
        mount: MountId,
        parent_mount: MountId,
    ) -> Option<InsertionSite> {
        // Child ownership is a committed-mount-table query. During a parent diff, the new vnode may
        // already have a different fragment shape, but the fragment child mount list is not
        // replaced until that parent diff commits.
        self.with_parent_vnode(parent_mount, ParentVersion::Old, |parent_vnode| {
            for slot in parent_vnode.dynamic_node_slots() {
                let idx = slot.index();
                match &parent_vnode.dynamic_node_values()[idx] {
                    DynamicNode::Fragment(children) => {
                        let site = self.dom.try_with_mounted_fragment_children(
                            parent_mount,
                            idx,
                            children.len(),
                            |child_mounts| {
                                let position =
                                    child_mounts.iter().position(|child| *child == mount)?;
                                self.resolve_fragment_child_site(
                                    children,
                                    child_mounts,
                                    position,
                                    mount,
                                )
                                .or_else(|| Some(self.resolve_slot(parent_mount, slot)))
                            },
                        );
                        if let Some(site) = site.flatten() {
                            return Some(site);
                        }
                    }
                    DynamicNode::Component(_)
                        if self
                            .dom
                            .mounted_dynamic_component_root_mount(parent_mount, idx)
                            == Some(mount) =>
                    {
                        return Some(self.resolve_slot(parent_mount, slot));
                    }
                    DynamicNode::Component(_) | DynamicNode::Text(_) => {}
                }
            }
            None
        })
    }

    fn resolve_fragment_child_site(
        &self,
        children: &[VNode],
        child_mounts: &[MountId],
        position: usize,
        mount: MountId,
    ) -> Option<InsertionSite> {
        let following = || {
            children
                .iter()
                .zip(child_mounts)
                .skip(position + 1)
                .filter(|(_, m)| **m != mount)
                .find_map(|(child, &m)| {
                    child
                        .find_first_element(m, self.dom)
                        .map(InsertionSite::before)
                })
        };

        let previous = || {
            children
                .iter()
                .zip(child_mounts)
                .take(position)
                .rev()
                .filter(|(_, m)| **m != mount)
                .find_map(|(child, &m)| {
                    child
                        .find_last_element(m, self.dom)
                        .map(InsertionSite::after)
                })
        };

        following().or_else(previous)
    }

    fn adjacent_dynamic_sibling_after_in_vnode(
        &self,
        parent_vnode: &VNode,
        parent_mount: MountId,
        slot: DynamicNodeSlot<'_>,
    ) -> Option<ElementId> {
        parent_vnode
            .dynamic_node_slots_after_sharing_insertion_position(slot)
            .find_map(|sibling| {
                parent_vnode.dynamic_node_edge_element(
                    parent_mount,
                    sibling.index(),
                    self.dom,
                    EdgeScan::placement(self.dom),
                    ElementEdge::First,
                )
            })
    }

    fn adjacent_dynamic_sibling_before_in_vnode(
        &self,
        parent_vnode: &VNode,
        parent_mount: MountId,
        slot: DynamicNodeSlot<'_>,
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
                self.dom,
                EdgeScan::placement(self.dom),
                ElementEdge::Last,
            ) {
                return Some(id);
            }
        }
        None
    }

    fn with_parent_vnode<T>(
        &self,
        parent_mount: MountId,
        version: ParentVersion,
        f: impl FnOnce(&VNode) -> Option<T>,
    ) -> Option<T> {
        if let Some(frame) = self
            .context
            .and_then(|context| context.for_mount(parent_mount))
        {
            let vnode = match version {
                ParentVersion::Old => frame.old,
                ParentVersion::New => frame.new,
            };
            f(vnode)
        } else {
            let vnode = self
                .dom
                .current_mounted_view(parent_mount)
                .expect("parent mount");
            f(&vnode)
        }
    }

    fn vnode_first_site(&self, vnode: MountedVNode<'_>) -> Option<InsertionSite> {
        vnode
            .find_first_element(self.dom)
            .map(InsertionSite::before)
    }

    fn anchor_site(&self, anchor: DynamicAnchor<'_>, anchor_id: ElementId) -> InsertionSite {
        if anchor.is_parent_append_target() {
            InsertionSite::append_to(anchor_id)
        } else if anchor.is_last_static_node() {
            InsertionSite::after(anchor_id)
        } else {
            InsertionSite::before(anchor_id)
        }
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

pub(crate) fn create_at_site(
    content: &VNode,
    parent: Option<MountId>,
    site: InsertionSite,
    dom: &mut VirtualDom,
    to: &mut dyn WriteMutations,
) -> CreatedVNode {
    at_site_with_result(site, to, dom.runtime.clone(), |to| {
        let created = content.create_mounted(dom, parent, parent, Some(to));
        (created.nodes, created)
    })
}

/// Like [`create_at_site`], but re-emits an already-mounted (background) subtree
/// at the site, reusing its existing mount and scopes instead of allocating fresh
/// ones. Used to promote a retained suspense branch to the foreground.
pub(crate) fn recreate_at_site(
    content: &VNode,
    mount: MountId,
    parent: Option<MountId>,
    site: InsertionSite,
    dom: &mut VirtualDom,
    to: &mut dyn WriteMutations,
) -> CreatedVNode {
    at_site_with_result(site, to, dom.runtime.clone(), |to| {
        let created = content.recreate_with_mount(dom, mount, parent, parent, Some(to));
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

pub(super) fn at_site_with_result<R>(
    site: InsertionSite,
    to: &mut dyn WriteMutations,
    runtime: Rc<Runtime>,
    create: impl FnOnce(&mut dyn WriteMutations) -> (usize, R),
) -> R {
    site.create_and_place_with_result(to, runtime, create)
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
    site: InsertionSite,
    push_nodes: impl FnOnce(&mut M) -> usize,
) -> usize {
    to.push_id(site.anchor());
    let count = push_nodes(to);
    // `insert_before`/`insert_after`/`append_children` leave the anchor on the stack, so pop it
    // to keep the renderer stack balanced - the discipline `TargetedLazyScope` applies on drop.
    site.splice_stack(to, count);
    to.pop();
    count
}

#[derive(Clone, Copy)]
enum ParentVersion {
    Old,
    New,
}
