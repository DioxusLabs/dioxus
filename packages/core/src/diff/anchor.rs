use std::rc::Rc;

use crate::{
    Runtime, TemplateCursor, TemplateNode, VNode, VirtualDom, WriteMutations,
    arena::ElementId,
    innerlude::{ElementRef, MountId},
    mutations::{LazyScope, append_children_to, insert_after_id, insert_before_id},
    nodes::DynamicNode,
};

use super::{
    context::DiffContext,
    template_path::{push_static_cursor, slot_appends, split_slot_cursor},
};

#[derive(Clone, Copy)]
pub(super) enum ElementEdge {
    First,
    Last,
}

/// A renderer-level position where `m` DOM nodes, already on the renderer stack,
/// should be spliced in.
#[derive(Debug, Clone)]
pub(crate) enum Anchor {
    Before(ElementId),
    After(ElementId),
    Slot {
        parent: ElementId,
        root: &'static TemplateNode,
        cursor_tail: &'static [u8],
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
            Anchor::Slot {
                parent,
                root,
                cursor_tail,
            } => insert_at_slot(to, *parent, root, cursor_tail, runtime, create),
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

pub(crate) fn anchor_for_slot(
    parent_mount: MountId,
    slot_id: usize,
    cursor: TemplateCursor,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Anchor {
    let cursor_slice = cursor.as_slice();
    // Every node cursor entry the diff hands us starts with the root index
    // (see `compile_template` and rsx codegen), so the empty cursor is
    // unreachable in practice.
    if cursor.is_root_level_slot() {
        let our_root_idx = cursor_slice[0] as usize;
        if let Some(id) =
            root_content_after_slot(parent_mount, slot_id, our_root_idx, skip, dom, context)
        {
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
    if let Some(enclosing) = dom.mounted_root_node(parent_mount, cursor_slice[0] as usize)
        && dom.element_exists_for_mount(parent_mount, enclosing)
    {
        if let Some(id) =
            adjacent_dynamic_sibling_after(parent_mount, slot_id, cursor, skip, dom, context)
        {
            return Anchor::Before(id);
        }
        let parent_view = context
            .and_then(|context| context.for_mount(parent_mount))
            .map_or_else(
                || {
                    dom.current_mounted_view(parent_mount)
                        .expect("slot parent must have a mounted view")
                },
                |context| context.new.clone(),
            );
        return Anchor::Slot {
            parent: enclosing.element_id(),
            root: &parent_view.template.roots()[cursor_slice[0] as usize],
            cursor_tail: &cursor_slice[1..],
        };
    }

    anchor_for_slot(
        parent_mount,
        slot_id,
        TemplateCursor::new(&cursor_slice[..1]),
        skip,
        dom,
        context,
    )
}

pub(crate) fn create_at_anchor(
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

pub(crate) fn at_anchor(
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
    root: &'static TemplateNode,
    cursor_tail: &'static [u8],
    runtime: Rc<Runtime>,
    create: impl FnOnce(&mut dyn WriteMutations) -> usize,
) -> usize {
    let (parent_cursor, slot_index) = split_slot_cursor(cursor_tail);
    let appends = slot_appends(root, parent_cursor, slot_index);
    let mut to = LazyScope::new_for_current_target(to, runtime, move |to| {
        to.push_id(root_id);
        push_static_cursor(to, root, parent_cursor);
        if !appends {
            to.child(slot_index as usize);
        }
    });
    let count = create(&mut to);
    if count > 0 {
        if appends {
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

    anchor_for_slot(parent_mount, slot_id, cursor, skip, dom, context)
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
    let same_view_anchor = parent_views.iter().find_map(|parent_vnode| {
        let children = fragment_children_for_slot(parent_vnode, slot_id)?;
        let position = locate_in_fragment(children, mount, key)?;
        first_live_sibling_after(children, position, mount, skip, dom)
    });
    if same_view_anchor.is_some() {
        return same_view_anchor;
    }

    let position = parent_views
        .iter()
        .filter_map(|parent_vnode| fragment_children_for_slot(parent_vnode, slot_id))
        .find_map(|children| locate_in_fragment(children, mount, None))?;

    parent_views
        .iter()
        .filter_map(|parent_vnode| fragment_children_for_slot(parent_vnode, slot_id))
        .filter(|children| key.is_none() || fragment_is_unkeyed(children))
        .find_map(|children| first_live_sibling_after(children, position, mount, skip, dom))
}

fn adjacent_dynamic_sibling_after(
    parent_mount: MountId,
    slot_id: usize,
    cursor: TemplateCursor,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Option<ElementId> {
    parent_views(dom, parent_mount, context)
        .iter()
        .find_map(|parent_vnode| {
            for (sibling_id, sibling_cursor) in parent_vnode
                .template
                .node_cursors()
                .iter()
                .copied()
                .enumerate()
                .skip(slot_id + 1)
            {
                if sibling_cursor != cursor {
                    break;
                }
                if let Some(anchor) =
                    first_live_dynamic_slot(parent_vnode, parent_mount, sibling_id, skip, dom)
                {
                    return Some(anchor);
                }
            }

            None
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
    match &vnode.dynamic_nodes[idx] {
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

fn parent_views(
    dom: &VirtualDom,
    parent_mount: MountId,
    context: Option<DiffContext<'_>>,
) -> Vec<VNode> {
    if let Some(context) = context.and_then(|context| context.for_mount(parent_mount)) {
        return vec![context.new.clone(), context.old.clone()];
    }
    dom.current_mounted_view(parent_mount).into_iter().collect()
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
    match &vnode.dynamic_nodes[slot_id] {
        DynamicNode::Fragment(children) => Some(children),
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
    slot_id: usize,
    our_root_idx: usize,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Option<ElementId> {
    // Probe the committed mount view of `parent_mount`. The diff context's
    // `old` snapshot matches the committed view by construction (both are
    // the pre-diff `mount.node`), so reading directly from the mount
    // registry covers both cases. Callers (`anchor_for_slot` after the
    // path-length check) only reach here with a `parent_mount` that's
    // currently being diffed, so its mount is registered.
    let _ = context;
    let probe = dom
        .current_mounted_view(parent_mount)
        .expect("parent_root_after requires a live parent mount");

    if let Some(id) =
        root_dynamic_content_after_slot(&probe, parent_mount, slot_id, our_root_idx, skip, dom)
    {
        return Some(id);
    }

    if let Some(id) = static_root_element(&probe, parent_mount, our_root_idx, dom) {
        return Some(id);
    }

    let upper = probe.template.roots().len();
    for next_cursor in (our_root_idx + 1)..=upper {
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
    slot_id: usize,
    cursor_idx: usize,
    skip: &[MountId],
    dom: &VirtualDom,
) -> Option<ElementId> {
    for (dynamic_id, cursor) in vnode
        .template
        .node_cursors()
        .iter()
        .copied()
        .enumerate()
        .skip(slot_id + 1)
    {
        if cursor.as_slice() != [cursor_idx as u8].as_slice() {
            break;
        }
        if let Some(id) = first_live_dynamic_slot(vnode, mount, dynamic_id, skip, dom) {
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
    let cursor = [cursor_idx as u8];
    let iter = vnode
        .template
        .node_cursors()
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, candidate)| candidate.as_slice() == cursor.as_slice());

    match edge {
        ElementEdge::First => iter
            .filter_map(|(idx, _)| first_live_dynamic_slot(vnode, mount, idx, skip, dom))
            .next(),
        ElementEdge::Last => iter
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .filter_map(|(idx, _)| last_live_dynamic_slot(vnode, mount, idx, skip, dom))
            .next(),
    }
}

pub(super) fn static_root_element(
    vnode: &VNode,
    mount: MountId,
    root_idx: usize,
    dom: &VirtualDom,
) -> Option<ElementId> {
    if root_idx >= vnode.template.roots().len() {
        return None;
    }
    dom.mounted_root_node(mount, root_idx)
        .filter(|id| dom.element_exists_for_mount(mount, *id))
        .map(|id| id.element_id())
}

fn last_live_dynamic_slot(
    vnode: &VNode,
    mount: MountId,
    idx: usize,
    skip: &[MountId],
    dom: &VirtualDom,
) -> Option<ElementId> {
    let target_id = dom.current_render_target_id();
    match &vnode.dynamic_nodes[idx] {
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
