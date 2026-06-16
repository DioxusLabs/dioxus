use std::rc::Rc;

use crate::{
    Runtime, VNode, VirtualDom, WriteMutations,
    arena::ElementId,
    innerlude::{ElementRef, MountId},
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

/// A renderer-level position where `m` DOM nodes, already on the renderer stack,
/// should be spliced in.
#[derive(Debug, Clone)]
pub(super) enum Anchor {
    Before(ElementId),
    After(ElementId),
    Slot {
        parent: ElementId,
        placement: SlotPlacement,
    },
    AppendTo(ElementId),
}

impl Anchor {
    fn create_and_place(
        &self,
        to: &mut dyn WriteMutations,
        runtime: Rc<Runtime>,
        create: impl FnOnce(&mut dyn WriteMutations) -> usize,
    ) -> usize {
        match self {
            Anchor::Before(id) => insert_before_id(to, *id, runtime, create),
            Anchor::After(id) => insert_after_id(to, *id, runtime, create),
            Anchor::AppendTo(id) => append_children_to(to, *id, runtime, create),
            Anchor::Slot { parent, placement } => {
                insert_at_slot(to, *parent, placement.clone(), runtime, create)
            }
        }
    }
}

/// Anchor new content at the given edge of `vnode`'s live DOM: before its
/// first element, or after its last. Falls back to the slot-level anchor when
/// the vnode has no live DOM.
pub(super) fn anchor_at(
    edge: ElementEdge,
    vnode: &VNode,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Anchor {
    let at_edge = match edge {
        ElementEdge::First => vnode.find_first_element(dom).map(Anchor::Before),
        ElementEdge::Last => vnode.find_last_element(dom).map(Anchor::After),
    };
    at_edge.unwrap_or_else(|| {
        anchor_for_with_key(
            vnode.unchecked_mounted_id(),
            vnode.key.as_deref(),
            skip,
            dom,
            context,
        )
    })
}

pub(super) fn anchor_for_slot(
    parent_mount: MountId,
    slot: DynamicNodeSlot<'_>,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Anchor {
    let root_idx = slot.root_index();
    // Every node cursor entry the diff hands us starts with the root index
    // (see `compile_template` and rsx codegen), so the empty cursor is
    // unreachable in practice.
    if slot.is_root_level() {
        let our_root_idx = root_idx;
        if let Some(id) = root_content_after_slot(parent_mount, slot, our_root_idx, skip, dom) {
            return Anchor::Before(id);
        }
        let parent_key = parent_key(parent_mount, dom, context);
        return anchor_for_with_key(parent_mount, parent_key.as_deref(), skip, dom, context);
    }

    // `cursor.len() > 1` means we're walking inside a template element, so
    // the parent vnode is always mounted and reachable from this diff
    // context. If the enclosing root has been reclaimed for any reason we
    // fall through to the slot-level anchor instead of trying to refer to a
    // stale element id.
    if let Some(enclosing) = dom.mounted_root_node(parent_mount, root_idx)
        && dom.element_exists_for_mount(parent_mount, enclosing)
    {
        if let Some(id) = adjacent_dynamic_sibling_after(parent_mount, slot, skip, dom, context) {
            return Anchor::Before(id);
        }
        return Anchor::Slot {
            parent: enclosing.element_id(),
            placement: slot.placement(),
        };
    }

    anchor_for_slot(parent_mount, slot.root_slot(), skip, dom, context)
}

pub(super) fn create_at_anchor(
    content: &[VNode],
    parent: Option<ElementRef>,
    anchor: Anchor,
    dom: &mut VirtualDom,
    to: Option<&mut dyn WriteMutations>,
) -> usize {
    at_anchor(anchor, to, dom.runtime.clone(), |to| {
        dom.create_children(to, content, parent)
    })
}

pub(super) fn at_anchor(
    anchor: Anchor,
    to: Option<&mut dyn WriteMutations>,
    runtime: Rc<Runtime>,
    create: impl FnOnce(Option<&mut dyn WriteMutations>) -> usize,
) -> usize {
    if let Some(to_ref) = to {
        anchor.create_and_place(to_ref, runtime, |to| create(Some(to)))
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
        if let Some(indexes) = &placement.static_child_indexes {
            for index in indexes.iter().copied() {
                to.child(index);
            }
        } else {
            for depth in 1..placement.parent_path.len() {
                to.child(placement.parent_path.segment(depth) as usize);
            }
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

fn anchor_for_with_key(
    mount: MountId,
    key: Option<&str>,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Anchor {
    let Some(parent_ref) = dom.get_mounted_parent(mount) else {
        return Anchor::AppendTo(ElementId::ROOT);
    };
    let parent_mount = parent_ref.mount;
    let Some((slot_id, cursor)) = parent_ref.location.slot() else {
        return Anchor::AppendTo(ElementId::ROOT);
    };

    if let Some(id) = fragment_sibling_after(mount, parent_mount, slot_id, key, skip, dom, context)
    {
        return Anchor::Before(id);
    }

    let parent_views = parent_views(dom, parent_mount, context);
    let Some(parent_view) = parent_views.first() else {
        return Anchor::AppendTo(ElementId::ROOT);
    };
    let slot = dynamic_slot_for(parent_view, slot_id)
        .unwrap_or_else(|| DynamicNodeSlot::new(&parent_view.template, slot_id, cursor));
    anchor_for_slot(parent_mount, slot, skip, dom, context)
}

fn fragment_sibling_after(
    mount: MountId,
    parent_mount: MountId,
    slot_id: usize,
    key: Option<&str>,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Option<ElementId> {
    let parent_views = parent_views(dom, parent_mount, context);
    let same_view_anchor = parent_views.find_map(|parent_vnode| {
        let children = fragment_children_for_slot(parent_vnode, slot_id)?;
        let position = locate_in_fragment(children, mount, key)?;
        first_live_sibling_after(children, position, mount, skip, dom)
    });
    if same_view_anchor.is_some() {
        return same_view_anchor;
    }

    let position = parent_views.find_map(|parent_vnode| {
        fragment_children_for_slot(parent_vnode, slot_id)
            .and_then(|children| locate_in_fragment(children, mount, None))
    })?;

    parent_views.find_map(|parent_vnode| {
        let children = fragment_children_for_slot(parent_vnode, slot_id)?;
        (key.is_none() || fragment_is_unkeyed(children))
            .then(|| first_live_sibling_after(children, position, mount, skip, dom))
            .flatten()
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
        let active_slot = dynamic_slot_for(parent_vnode, slot.index()).unwrap_or_else(|| slot);
        let mut anchor = None;
        for sibling in dynamic_node_slots(parent_vnode) {
            if sibling.index() <= active_slot.index() {
                continue;
            }
            if sibling.parent_path() == active_slot.parent_path()
                && !sibling.shares_insertion_position(active_slot)
            {
                break;
            }
            if !sibling.shares_insertion_position(active_slot) {
                continue;
            }
            if let Some(id) =
                first_live_dynamic_slot(parent_vnode, parent_mount, sibling.index(), skip, dom)
            {
                anchor = Some(id);
                break;
            }
        }

        anchor
    })
}

fn first_live_dynamic_slot(
    vnode: &VNode,
    mount: MountId,
    idx: usize,
    skip: &[MountId],
    dom: &VirtualDom,
) -> Option<ElementId> {
    let target_id = dom.current_render_target_id();
    match vnode.dynamic_values[idx].node() {
        DynamicNode::Text(_) => dom
            .mounted_dynamic_text_node(mount, idx)
            .filter(|id| dom.element_exists_in_target(target_id, *id))
            .map(|id| id.element_id()),
        DynamicNode::Fragment(children) => children.iter().find_map(|child| {
            let mount = child.mounted_id()?;
            if skip.contains(&mount) {
                return None;
            }
            child.find_first_element(dom)
        }),
        DynamicNode::Component(_) => {
            let scope_id = dom.mounted_dynamic_component_scope(mount, idx)?;
            let root = dom.get_scope(scope_id)?.try_root_node()?;
            let mount = root.mounted_id()?;
            if skip.contains(&mount) {
                return None;
            }
            root.find_first_element(dom)
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
    fn first(&self) -> Option<&VNode> {
        match self {
            Self::Context { new, .. } => Some(new),
            Self::Mounted(view) => view.as_ref(),
        }
    }

    fn find_map<T>(&self, mut f: impl FnMut(&VNode) -> Option<T>) -> Option<T> {
        match self {
            Self::Context { new, old } => f(new).or_else(|| f(old)),
            Self::Mounted(view) => view.as_ref().and_then(f),
        }
    }
}

fn parent_key(
    parent_mount: MountId,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Option<String> {
    context
        .and_then(|context| context.for_mount(parent_mount))
        .and_then(|context| context.new.key.clone())
        .or_else(|| {
            dom.current_mounted_view(parent_mount)
                .and_then(|v| v.key.clone())
        })
}

fn fragment_is_unkeyed(children: &[VNode]) -> bool {
    children
        .first()
        .is_none_or(|child| child.key.as_deref().is_none())
}

fn fragment_children_for_slot(vnode: &VNode, slot_id: usize) -> Option<&[VNode]> {
    match vnode.dynamic_values[slot_id].node() {
        DynamicNode::Fragment(children) => Some(children.as_slice()),
        _ => None,
    }
}

fn locate_in_fragment(children: &[VNode], mount: MountId, key: Option<&str>) -> Option<usize> {
    key.and_then(|k| children.iter().position(|c| c.key.as_deref() == Some(k)))
        .or_else(|| children.iter().position(|c| c.mounted_id() == Some(mount)))
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
    // registry covers both cases. Callers (`anchor_for_slot` after the
    // path-length check) only reach here with a `parent_mount` that's
    // currently being diffed, so its mount is registered.
    let probe = dom
        .current_mounted_view(parent_mount)
        .expect("parent_root_after requires a live parent mount");

    if let Some(id) =
        root_dynamic_content_after_slot(&probe, parent_mount, slot, our_root_idx, skip, dom)
    {
        return Some(id);
    }

    if let Some(id) = static_root_element(&probe, parent_mount, our_root_idx, dom) {
        return Some(id);
    }

    let upper = probe.template.root_count();
    for next_cursor in (our_root_idx + 1)..upper {
        if let Some(id) = first_root_dynamic_at_cursor(
            &probe,
            parent_mount,
            next_cursor,
            skip,
            dom,
            ElementEdge::First,
        ) {
            return Some(id);
        }
        if let Some(id) = static_root_element(&probe, parent_mount, next_cursor, dom) {
            return Some(id);
        }
    }
    None
}

fn root_dynamic_content_after_slot(
    vnode: &VNode,
    mount: MountId,
    slot: DynamicNodeSlot<'_>,
    cursor_idx: usize,
    skip: &[MountId],
    dom: &VirtualDom,
) -> Option<ElementId> {
    for candidate in dynamic_node_slots(vnode) {
        if candidate.index() <= slot.index() {
            continue;
        }
        if !candidate.is_root_level() || candidate.root_index() != cursor_idx {
            break;
        }
        if let Some(id) = first_live_dynamic_slot(vnode, mount, candidate.index(), skip, dom) {
            return Some(id);
        }
    }
    None
}

pub(super) fn first_root_dynamic_at_cursor(
    vnode: &VNode,
    mount: MountId,
    cursor_idx: usize,
    skip: &[MountId],
    dom: &VirtualDom,
    edge: ElementEdge,
) -> Option<ElementId> {
    let mut found = None;
    match edge {
        ElementEdge::First => {
            for slot in dynamic_node_slots(vnode) {
                if !slot.is_root_level() || slot.root_index() != cursor_idx {
                    continue;
                }
                found = first_live_dynamic_slot(vnode, mount, slot.index(), skip, dom);
                if found.is_some() {
                    break;
                }
            }
            found
        }
        ElementEdge::Last => {
            for slot in dynamic_node_slots(vnode) {
                if slot.is_root_level() && slot.root_index() == cursor_idx {
                    found = last_live_dynamic_slot(vnode, mount, slot.index(), skip, dom);
                }
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

fn last_live_dynamic_slot(
    vnode: &VNode,
    mount: MountId,
    idx: usize,
    skip: &[MountId],
    dom: &VirtualDom,
) -> Option<ElementId> {
    let target_id = dom.current_render_target_id();
    match vnode.dynamic_values[idx].node() {
        DynamicNode::Text(_) => dom
            .mounted_dynamic_text_node(mount, idx)
            .filter(|id| dom.element_exists_in_target(target_id, *id))
            .map(|id| id.element_id()),
        DynamicNode::Fragment(children) => children.iter().rev().find_map(|child| {
            let mount = child.mounted_id()?;
            if skip.contains(&mount) {
                return None;
            }
            child.find_last_element(dom)
        }),
        DynamicNode::Component(_) => {
            let scope_id = dom.mounted_dynamic_component_scope(mount, idx)?;
            let root = dom.get_scope(scope_id)?.try_root_node()?;
            let mount = root.mounted_id()?;
            if skip.contains(&mount) {
                return None;
            }
            root.find_last_element(dom)
        }
    }
}
