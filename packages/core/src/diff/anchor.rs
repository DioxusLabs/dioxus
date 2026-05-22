use crate::{
    VNode, VirtualDom, WriteMutations,
    arena::ElementId,
    innerlude::{ElementRef, MountId},
    nodes::DynamicNode,
};
use core::mem::discriminant;

use super::context::DiffContext;

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
        path: &'static [u8],
    },
    AppendTo(ElementId),
}

impl Anchor {
    fn place(&self, m: usize, to: &mut impl WriteMutations) {
        if m == 0 {
            return;
        }
        match self {
            Anchor::Before(id) => to.insert_nodes_before(*id, m),
            Anchor::After(id) => to.insert_nodes_after(*id, m),
            Anchor::AppendTo(id) => to.append_children(*id, m),
            Anchor::Slot { path, .. } => to.insert_children_at_path(path, m),
        }
    }
}

pub(crate) fn anchor_before(
    vnode: &VNode,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Anchor {
    vnode
        .find_first_element(dom)
        .map(Anchor::Before)
        .unwrap_or_else(|| {
            anchor_for_with_key(vnode.mount.get(), vnode.key.as_deref(), skip, dom, context)
        })
}

pub(crate) fn anchor_after(
    vnode: &VNode,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Anchor {
    vnode
        .find_last_element(dom)
        .map(Anchor::After)
        .unwrap_or_else(|| {
            anchor_for_with_key(vnode.mount.get(), vnode.key.as_deref(), skip, dom, context)
        })
}

pub(crate) fn anchor_for_slot(
    parent_mount: MountId,
    path: &'static [u8],
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Anchor {
    if path.is_empty() {
        return anchor_for_with_key(parent_mount, None, skip, dom, context);
    }

    if path.len() == 1 {
        let our_root_idx = path[0] as usize;
        if let Some(id) = parent_root_after(parent_mount, our_root_idx, dom, context) {
            return Anchor::Before(id);
        }
        let parent_key = parent_key(parent_mount, dom, context);
        return anchor_for_with_key(parent_mount, parent_key.as_deref(), skip, dom, context);
    }

    if parent_mount.mounted() && has_parent_view(parent_mount, dom, context) {
        let enclosing = dom.get_mounted_root_node(parent_mount, path[0] as usize);
        if enclosing != ElementId::default()
            && dom.element_exists_for_mount(parent_mount, enclosing)
        {
            return Anchor::Slot {
                parent: enclosing,
                path: &path[1..],
            };
        }
        debug_assert!(
            enclosing == ElementId::default()
                || !dom.element_exists_for_mount(parent_mount, enclosing),
            "nested slot anchor pointed at stale root {enclosing:?}"
        );
    } else {
        debug_assert!(
            parent_mount.mounted() && has_parent_view(parent_mount, dom, context),
            "anchor_for_slot called with stale parent mount {parent_mount:?}"
        );
    }

    anchor_for_slot(parent_mount, &path[..1], skip, dom, context)
}

pub(crate) fn create_at_anchor(
    content: &[VNode],
    parent: Option<ElementRef>,
    anchor: Anchor,
    dom: &mut VirtualDom,
    to: Option<&mut impl WriteMutations>,
) -> usize {
    at_anchor(anchor, to, |to| dom.create_children(to, content, parent))
}

pub(crate) fn create_at_anchor_with_parents(
    content: &[VNode],
    render_parent: Option<ElementRef>,
    logical_parent: Option<ElementRef>,
    anchor: Anchor,
    dom: &mut VirtualDom,
    to: Option<&mut impl WriteMutations>,
) -> usize {
    at_anchor(anchor, to, |to| {
        dom.create_children_with_parents(to, content, render_parent, logical_parent)
    })
}

pub(crate) fn at_anchor<M: WriteMutations>(
    anchor: Anchor,
    mut to: Option<&mut M>,
    create: impl FnOnce(Option<&mut M>) -> usize,
) -> usize {
    let stack_parent = match &anchor {
        Anchor::Slot { parent, .. } => Some(*parent),
        _ => None,
    };
    if let Some(parent) = stack_parent {
        if let Some(to_ref) = to.as_deref_mut() {
            to_ref.push_root(parent);
        }
    }
    let m = create(to.as_deref_mut());
    if let Some(to_ref) = to.as_deref_mut() {
        anchor.place(m, to_ref);
    }
    if stack_parent.is_some() {
        if let Some(to_ref) = to {
            to_ref.pop_root();
        }
    }
    m
}

fn anchor_for_with_key(
    mount: MountId,
    key: Option<&str>,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Anchor {
    let Some(parent_ref) = dom.get_mounted_parent(mount) else {
        debug_assert!(
            !mount.mounted() || has_parent_view(mount, dom, context),
            "missing parent for stale mounted node {mount:?}"
        );
        return Anchor::AppendTo(ElementId::ROOT);
    };
    let parent_mount = parent_ref.mount;
    let path = parent_ref.path.path;
    if path.is_empty() {
        if let Some(id) = fragment_sibling_after(mount, parent_mount, path, key, skip, dom, context)
        {
            return Anchor::Before(id);
        }
        debug_assert!(
            dom.get_mounted_parent(parent_mount).is_none(),
            "empty parent path should only be used below the root mount"
        );
        return Anchor::AppendTo(ElementId::ROOT);
    }

    if let Some(id) = fragment_sibling_after(mount, parent_mount, path, key, skip, dom, context) {
        return Anchor::Before(id);
    }

    anchor_for_slot(parent_mount, path, skip, dom, context)
}

fn fragment_sibling_after(
    mount: MountId,
    parent_mount: MountId,
    path: &'static [u8],
    key: Option<&str>,
    skip: &[MountId],
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Option<ElementId> {
    let parent_views = parent_views(dom, parent_mount, context);
    let same_view_anchor = parent_views.iter().find_map(|parent_vnode| {
        let children = fragment_children_at_path(parent_vnode, path)?;
        let position = locate_in_fragment(children, mount, key)?;
        first_live_sibling_after(children, position, mount, skip, dom)
    });
    if same_view_anchor.is_some() {
        return same_view_anchor;
    }

    let position = parent_views
        .iter()
        .filter_map(|parent_vnode| fragment_children_at_path(parent_vnode, path))
        .find_map(|children| locate_in_fragment(children, mount, None))?;

    parent_views
        .iter()
        .filter_map(|parent_vnode| fragment_children_at_path(parent_vnode, path))
        .filter(|children| key.is_none() || fragment_is_unkeyed(children))
        .find_map(|children| first_live_sibling_after(children, position, mount, skip, dom))
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

fn has_parent_view(
    parent_mount: MountId,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> bool {
    context
        .and_then(|context| context.for_mount(parent_mount))
        .is_some()
        || dom.current_mounted_view(parent_mount).is_some()
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

fn fragment_children_at_path<'a>(vnode: &'a VNode, path: &'static [u8]) -> Option<&'a [VNode]> {
    let dyn_id = vnode
        .template
        .node_paths()
        .iter()
        .position(|p| *p == path)?;
    match &vnode.dynamic_nodes[dyn_id] {
        DynamicNode::Fragment(children) => Some(children),
        _ => None,
    }
}

fn locate_in_fragment(children: &[VNode], mount: MountId, key: Option<&str>) -> Option<usize> {
    key.and_then(|k| children.iter().position(|c| c.key.as_deref() == Some(k)))
        .or_else(|| children.iter().position(|c| c.mount.get() == mount))
}

fn first_live_sibling_after(
    children: &[VNode],
    position: usize,
    mount: MountId,
    skip: &[MountId],
    dom: &VirtualDom,
) -> Option<ElementId> {
    children.iter().skip(position + 1).find_map(|child| {
        let m = child.mount.get();
        if !m.mounted() || skip.contains(&m) || m == mount {
            return None;
        }
        child.find_first_element(dom)
    })
}

fn parent_root_after(
    parent_mount: MountId,
    our_root_idx: usize,
    dom: &VirtualDom,
    context: Option<DiffContext<'_>>,
) -> Option<ElementId> {
    let context = context.and_then(|context| context.for_mount(parent_mount));
    let current = context
        .map(|context| context.old.clone())
        .or_else(|| dom.current_mounted_view(parent_mount));
    let next_view = context.map(|context| context.new.clone());
    let upper = current
        .iter()
        .chain(next_view.iter())
        .map(|v| v.template.roots().len())
        .max()?;

    for next in (our_root_idx + 1)..upper {
        if let Some(current) = current.as_ref().filter(|v| next < v.template.roots().len()) {
            if let Some(id) =
                current.find_element_at_root_via_mount(next, parent_mount, dom, ElementEdge::First)
            {
                return Some(id);
            }
        }

        let Some(next_view) = next_view
            .as_ref()
            .filter(|v| next < v.template.roots().len())
        else {
            continue;
        };
        if current
            .as_ref()
            .is_some_and(|current| !root_mount_shape_matches(current, next_view, next))
        {
            continue;
        }
        if let Some(id) =
            next_view.find_element_at_root_via_mount(next, parent_mount, dom, ElementEdge::First)
        {
            return Some(id);
        }
    }
    None
}

fn root_mount_shape_matches(old: &VNode, current: &VNode, root_idx: usize) -> bool {
    match (
        old.get_dynamic_root_node_and_id(root_idx),
        current.get_dynamic_root_node_and_id(root_idx),
    ) {
        (None, None) => true,
        (Some((_, old)), Some((_, current))) => discriminant(old) == discriminant(current),
        _ => false,
    }
}
