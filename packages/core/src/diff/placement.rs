use std::rc::Rc;

use crate::{
    Runtime, VNode, VirtualDom, WriteMutations,
    arena::ElementId,
    innerlude::{MountId, MountRef},
    mutations::{LazyScope, append_children_to, insert_after_id, insert_before_id},
    nodes::DynamicNode,
};

use super::{
    context::DiffContext,
    template::{DynamicNodeSlot, SlotPlacement, dynamic_node_slot, dynamic_node_slots},
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
#[derive(Debug, Clone)]
pub(super) enum DomAnchor {
    Before(ElementId),
    After(ElementId),
}

/// A renderer-level insertion site for nodes already on the renderer stack.
#[derive(Debug, Clone)]
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
            InsertionSite::AtAnchor(DomAnchor::Before(id)) => {
                insert_before_id(to, *id, runtime, create)
            }
            InsertionSite::AtAnchor(DomAnchor::After(id)) => {
                insert_after_id(to, *id, runtime, create)
            }
            InsertionSite::AppendTo(id) => append_children_to(to, *id, runtime, create),
            InsertionSite::Slot { parent, placement } => {
                insert_at_slot(to, *parent, placement.clone(), runtime, create)
            }
        }
    }
}

/// Find an insertion site at the given edge of `vnode`'s live DOM: before its
/// first element, or after its last. Falls back through mounted parent slots
/// when the vnode has no live DOM.
pub(super) fn insertion_site_at(
    edge: ElementEdge,
    vnode: &VNode,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> InsertionSite {
    let at_edge =
        vnode_edge_element(vnode, dom, edge).map(|id| InsertionSite::AtAnchor(edge.anchor(id)));
    at_edge.unwrap_or_else(|| {
        insertion_site_for_mounted_child(vnode.unchecked_mounted_id(), skip, dom, context)
    })
}

pub(super) fn insertion_site_for_slot(
    parent_mount: MountId,
    slot: DynamicNodeSlot<'_>,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> InsertionSite {
    let root_idx = slot.root_index();
    // Every node cursor entry the diff hands us starts with the root index
    // (see `compile_template` and rsx codegen), so the empty cursor is
    // unreachable in practice.
    if slot.is_root_level() {
        let our_root_idx = root_idx;
        if let Some(id) = root_content_after_slot(parent_mount, slot, our_root_idx, skip, dom) {
            return InsertionSite::AtAnchor(DomAnchor::Before(id));
        }
        return insertion_site_for_mounted_child(parent_mount, skip, dom, context);
    }

    // `cursor.len() > 1` means we're walking inside a template element, so
    // the parent vnode is always mounted and reachable from this diff
    // context. If the enclosing root has been reclaimed for any reason we
    // fall through to the slot-level insertion site instead of trying to refer to a
    // stale element id.
    if let Some(enclosing) = dom.mounted_root_node(parent_mount, root_idx)
        && dom.element_exists_for_mount(parent_mount, enclosing)
    {
        if let Some(id) = adjacent_dynamic_sibling_after(parent_mount, slot, skip, dom, context) {
            return InsertionSite::AtAnchor(DomAnchor::Before(id));
        }
        return InsertionSite::Slot {
            parent: enclosing.element_id(),
            placement: slot.placement(),
        };
    }

    insertion_site_for_slot(parent_mount, slot.root_slot(), skip, dom, context)
}

pub(super) fn insertion_site_for_loaded_static_slot(
    parent_mount: MountId,
    root_idx: usize,
    slot: DynamicNodeSlot<'_>,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> InsertionSite {
    if let Some(enclosing) = dom.mounted_root_node(parent_mount, root_idx)
        && dom.element_exists_for_mount(parent_mount, enclosing)
    {
        return InsertionSite::Slot {
            parent: enclosing.element_id(),
            placement: slot.placement(),
        };
    }

    insertion_site_for_slot(parent_mount, slot, &[], dom, context)
}

pub(super) fn create_at_site(
    content: &[VNode],
    parent: Option<MountRef>,
    site: InsertionSite,
    dom: &mut VirtualDom,
    to: Option<&mut dyn WriteMutations>,
) -> usize {
    at_site(site, to, dom.runtime.clone(), |to| {
        dom.create_children(to, content, parent)
    })
}

pub(super) fn at_site(
    site: InsertionSite,
    to: Option<&mut dyn WriteMutations>,
    runtime: Rc<Runtime>,
    create: impl FnOnce(Option<&mut dyn WriteMutations>) -> usize,
) -> usize {
    if let Some(to_ref) = to {
        site.create_and_place(to_ref, runtime, |to| create(Some(to)))
    } else {
        create(None)
    }
}

fn insert_at_slot(
    to: &mut dyn WriteMutations,
    root_id: ElementId,
    placement: SlotPlacement,
    runtime: Rc<Runtime>,
    create: impl FnOnce(&mut dyn WriteMutations) -> usize,
) -> usize {
    let mut to = LazyScope::new_for_current_target(to, runtime, move |to| {
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
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> InsertionSite {
    let Some(parent_ref) = dom.get_mounted_parent(mount) else {
        return InsertionSite::AppendTo(ElementId::ROOT);
    };
    let parent_mount = parent_ref.mount;

    if let Some(site) = insertion_site_for_child_in_parent(mount, parent_mount, skip, dom, context)
    {
        return site;
    }

    insertion_site_for_mounted_child(parent_mount, skip, dom, context)
}

fn insertion_site_for_child_in_parent(
    mount: MountId,
    parent_mount: MountId,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Option<InsertionSite> {
    let parent_views = parent_views(dom, parent_mount, context);
    parent_views.find_map(|parent_vnode| {
        for (idx, _) in parent_vnode.template.node_paths() {
            let slot = dynamic_slot_for(parent_vnode, idx)?;
            match parent_vnode.dynamic_values[idx].node() {
                DynamicNode::Fragment(children) => {
                    let position = locate_in_fragment(children, mount)?;
                    if let Some(id) = first_live_sibling_after(children, position, mount, skip, dom)
                    {
                        return Some(InsertionSite::AtAnchor(DomAnchor::Before(id)));
                    }
                    return Some(insertion_site_for_slot(
                        parent_mount,
                        slot,
                        skip,
                        dom,
                        context,
                    ));
                }
                DynamicNode::Component(_) => {
                    if dom.mounted_dynamic_component_root_mount(parent_mount, idx) == Some(mount) {
                        return Some(insertion_site_for_slot(
                            parent_mount,
                            slot,
                            skip,
                            dom,
                            context,
                        ));
                    }
                }
                DynamicNode::Text(_) => {}
            }
        }
        None
    })
}

fn adjacent_dynamic_sibling_after(
    parent_mount: MountId,
    slot: DynamicNodeSlot<'_>,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Option<ElementId> {
    parent_views(dom, parent_mount, context).find_map(|parent_vnode| {
        adjacent_dynamic_sibling_after_in_vnode(parent_vnode, parent_mount, slot, skip, dom)
    })
}

fn adjacent_dynamic_sibling_after_in_vnode(
    parent_vnode: &VNode,
    parent_mount: MountId,
    slot: DynamicNodeSlot<'_>,
    skip: &[MountId],
    dom: &VirtualDom,
) -> Option<ElementId> {
    let active_slot = dynamic_slot_for(parent_vnode, slot.index()).unwrap_or(slot);
    first_live_slot_after(
        parent_vnode,
        parent_mount,
        active_slot.index(),
        skip,
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
    skip: &[MountId],
    dom: &VirtualDom,
    mut scan: impl FnMut(DynamicNodeSlot<'_>) -> Option<bool>,
) -> Option<ElementId> {
    for slot in dynamic_node_slots(vnode) {
        if slot.index() <= after_idx {
            continue;
        }
        match scan(slot) {
            Some(true) => {
                if let Some(id) = live_dynamic_slot_edge(
                    vnode,
                    mount,
                    slot.index(),
                    skip,
                    dom,
                    ElementEdge::First,
                ) {
                    return Some(id);
                }
            }
            Some(false) => {}
            None => break,
        }
    }
    None
}

fn live_dynamic_slot_edge(
    vnode: &VNode,
    mount: MountId,
    idx: usize,
    skip: &[MountId],
    dom: &VirtualDom,
    edge: ElementEdge,
) -> Option<ElementId> {
    let target_id = dom.current_render_target_id();
    match vnode.dynamic_values[idx].node() {
        DynamicNode::Text(_) => dom
            .mounted_dynamic_text_node(mount, idx)
            .filter(|id| dom.element_exists_in_target(target_id, *id))
            .map(|id| id.element_id()),
        DynamicNode::Fragment(children) => edge.find_map(children.len(), |idx| {
            let child = &children[idx];
            let child_mount = child.mounted_id()?;
            if skip.contains(&child_mount) {
                return None;
            }
            vnode_edge_element(child, dom, edge)
        }),
        DynamicNode::Component(_) => {
            let component_root_mount = dom.mounted_dynamic_component_root_mount(mount, idx)?;
            if skip.contains(&component_root_mount) {
                return None;
            }
            let vnode = dom.current_mounted_view(component_root_mount)?;
            vnode_edge_element(&vnode, dom, edge)
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
            new: context.new,
            old: context.old,
        };
    }
    ParentViews::Mounted(dom.current_mounted_view(parent_mount))
}

enum ParentViews<'a> {
    Context { new: &'a VNode, old: &'a VNode },
    Mounted(Option<VNode>),
}

impl<'a> ParentViews<'a> {
    fn find_map<T>(&self, mut f: impl FnMut(&VNode) -> Option<T>) -> Option<T> {
        match self {
            Self::Context { new, old } => f(new).or_else(|| f(old)),
            Self::Mounted(view) => view.as_ref().and_then(f),
        }
    }
}

fn locate_in_fragment(children: &[VNode], mount: MountId) -> Option<usize> {
    children
        .iter()
        .position(|child| child.mounted_id() == Some(mount))
}

fn first_live_sibling_after(
    children: &[VNode],
    position: usize,
    mount: MountId,
    skip: &[MountId],
    dom: &VirtualDom,
) -> Option<ElementId> {
    children.iter().skip(position + 1).find_map(|child| {
        let m = child.mounted_id()?;
        if skip.contains(&m) || m == mount {
            return None;
        }
        child.find_first_element(dom)
    })
}

fn root_content_after_slot(
    parent_mount: MountId,
    slot: DynamicNodeSlot<'_>,
    our_root_idx: usize,
    skip: &[MountId],
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

    if let Some(id) =
        first_live_slot_after(&probe, parent_mount, slot.index(), skip, dom, |candidate| {
            (candidate.is_root_level() && candidate.root_index() == our_root_idx).then_some(true)
        })
    {
        return Some(id);
    }

    if let Some(id) = static_root_element(&probe, parent_mount, our_root_idx, dom) {
        return Some(id);
    }

    ((our_root_idx + 1)..probe.template.root_count()).find_map(|next_cursor| {
        first_root_dynamic_at_cursor(
            &probe,
            parent_mount,
            next_cursor,
            skip,
            dom,
            ElementEdge::First,
        )
        .or_else(|| static_root_element(&probe, parent_mount, next_cursor, dom))
    })
}

pub(super) fn first_root_dynamic_at_cursor(
    vnode: &VNode,
    mount: MountId,
    cursor_idx: usize,
    skip: &[MountId],
    dom: &VirtualDom,
    edge: ElementEdge,
) -> Option<ElementId> {
    find_root_dynamic_slot(vnode, cursor_idx, edge, |slot| {
        live_dynamic_slot_edge(vnode, mount, slot.index(), skip, dom, edge)
    })
}

pub(super) fn find_root_dynamic_slot<T>(
    vnode: &VNode,
    cursor_idx: usize,
    edge: ElementEdge,
    mut find: impl FnMut(DynamicNodeSlot<'_>) -> Option<T>,
) -> Option<T> {
    let mut slots = dynamic_node_slots(vnode)
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
    if root_idx >= vnode.template.root_count() {
        return None;
    }
    dom.mounted_root_node(mount, root_idx)
        .filter(|id| dom.element_exists_for_mount(mount, *id))
        .map(|id| id.element_id())
}

fn dynamic_slot_for(vnode: &VNode, slot_id: usize) -> Option<DynamicNodeSlot<'_>> {
    dynamic_node_slot(vnode, slot_id)
}

fn vnode_edge_element(vnode: &VNode, dom: &VirtualDom, edge: ElementEdge) -> Option<ElementId> {
    match edge {
        ElementEdge::First => vnode.find_first_element(dom),
        ElementEdge::Last => vnode.find_last_element(dom),
    }
}
