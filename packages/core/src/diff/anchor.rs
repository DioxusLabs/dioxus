use crate::{
    VNode, VirtualDom, WriteMutations,
    arena::ElementId,
    innerlude::{ElementRef, MountId},
    nodes::DynamicNode,
};

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
            Anchor::Slot { parent, path } => to.insert_children_at_path(*parent, path, m),
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
    // Every `node_paths` entry the diff hands us starts with the root index
    // (see `compile_template` and rsx codegen), so the empty path is
    // unreachable in practice.
    if path.len() == 1 {
        let our_root_idx = path[0] as usize;
        if let Some(id) = parent_root_after(parent_mount, our_root_idx, dom, context) {
            return Anchor::Before(id);
        }
        let parent_key = parent_key(parent_mount, dom, context);
        return anchor_for_with_key(parent_mount, parent_key.as_deref(), skip, dom, context);
    }

    // `path.len() > 1` means we're walking inside a template element, so
    // the parent vnode is always mounted and reachable from this diff
    // context. If the enclosing root has been reclaimed for any reason we
    // fall through to the slot-level anchor instead of trying to refer to a
    // stale element id.
    let enclosing = dom.get_mounted_root_node(parent_mount, path[0] as usize);
    if enclosing != ElementId::default() && dom.element_exists_for_mount(parent_mount, enclosing) {
        return Anchor::Slot {
            parent: enclosing,
            path: &path[1..],
        };
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

pub(crate) fn at_anchor<M: WriteMutations>(
    anchor: Anchor,
    mut to: Option<&mut M>,
    create: impl FnOnce(Option<&mut M>) -> usize,
) -> usize {
    let m = create(to.as_deref_mut());
    if let Some(to_ref) = to {
        anchor.place(m, to_ref);
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
        return Anchor::AppendTo(ElementId::ROOT);
    };
    let parent_mount = parent_ref.mount;
    // Same invariant as `anchor_for_slot`: every `ElementRef::path` is built
    // from a `template.node_paths()` entry, which always begins with the
    // root index, so it is never empty.
    let path = parent_ref.path.path;

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
    // `path` is sourced from an `ElementRef` that was constructed against
    // this vnode's template during the current diff, so it always corresponds
    // to a dynamic slot in `node_paths()`. The position lookup therefore
    // always succeeds here — slot mismatches would only arise from a
    // mid-flight template swap, which the diff completes atomically per
    // parent mount.
    let dyn_id = vnode
        .template
        .node_paths()
        .iter()
        .position(|p| *p == path)
        .expect("fragment path must resolve to a dynamic slot in the vnode's template");
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
    let upper = probe.template.roots().len();
    for next in (our_root_idx + 1)..upper {
        if let Some(id) =
            probe.find_element_at_root_via_mount(next, parent_mount, dom, ElementEdge::First)
        {
            return Some(id);
        }
    }
    None
}
