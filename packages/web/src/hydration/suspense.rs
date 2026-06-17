use crate::dom::WebsysDom;
use dioxus_core::{
    DynamicNode, ElementId, MountedVNode, ScopeId, ScopeState, SuspenseContext, VirtualDom,
    internal::TemplateExt,
};
use std::fmt::Write;

#[derive(Debug)]
struct SuspenseHydrationIdsNode {
    /// The scope id of the suspense boundary
    scope_id: ScopeId,
    /// Children of this node
    children: Vec<SuspenseHydrationIdsNode>,
}

impl SuspenseHydrationIdsNode {
    fn new(scope_id: ScopeId) -> Self {
        Self {
            scope_id,
            children: Vec::new(),
        }
    }

    fn traverse(&self, path: &[u32]) -> Option<&Self> {
        match path {
            [] => Some(self),
            [id, rest @ ..] => self.children.get(*id as usize)?.traverse(rest),
        }
    }

    fn traverse_mut(&mut self, path: &[u32]) -> Option<&mut Self> {
        match path {
            [] => Some(self),
            [id, rest @ ..] => self.children.get_mut(*id as usize)?.traverse_mut(rest),
        }
    }
}

/// Streaming hydration happens in waves. The server assigns suspense hydrations ids based on the order
/// the suspense boundaries are discovered in which should be consistent on the client and server.
///
/// This struct keeps track of the order the suspense boundaries are discovered in on the client so we can map the id in the dom to the scope we need to rehydrate.
///
/// Diagram: <https://excalidraw.com/#json=4NxmW90g0207Y62lESxfF,vP_Yn6j7k23utq2HZIsuiw>
#[derive(Default, Debug)]
pub(crate) struct SuspenseHydrationIds {
    /// A dense mapping from traversal order to the scope id of the suspense boundary
    /// The suspense boundary may be unmounted if the component was removed after partial hydration on the client
    children: Vec<SuspenseHydrationIdsNode>,
    pub(super) current_path: Vec<u32>,
}

impl SuspenseHydrationIds {
    /// Add a suspense boundary to the list of suspense boundaries. This should only be called on the root scope after the first rebuild (which happens on the server) and on suspense boundaries that are resolved from the server.
    /// Running this on a scope that is only created on the client may cause hydration issues.
    pub(super) fn add_suspense_boundary(&mut self, id: ScopeId) {
        match self.current_path.as_slice() {
            // This is a root node, add the new node
            [] => {
                self.children.push(SuspenseHydrationIdsNode::new(id));
            }
            // This isn't a root node, traverse into children and add the new node
            [first_index, rest @ ..] => {
                let child_node = self.children[*first_index as usize]
                    .traverse_mut(rest)
                    .unwrap();
                child_node.children.push(SuspenseHydrationIdsNode::new(id));
            }
        }
    }

    /// Get the scope id of the suspense boundary from the id in the dom
    pub(super) fn get_suspense_boundary(&self, path: &[u32]) -> Option<ScopeId> {
        let root = self.children.get(path[0] as usize)?;
        root.traverse(&path[1..]).map(|node| node.scope_id)
    }
}

/// Locate the ElementId of the first leaf in a scope's render that should be
/// bound to the empty-chunk bootstrap sentinel. Walks roots in order; for
/// each root, if it's a static element/text the root_id is returned, if it's
/// a Dynamic node we recurse into the dynamic node. Returns the FIRST
/// ElementId encountered — for the error path, this is the placeholder the
/// fallback ultimately collapses to.
pub(super) fn first_dynamic_root_element_id(
    scope: &ScopeState,
    dom: &VirtualDom,
) -> Option<ElementId> {
    fn from_vnode(vnode: MountedVNode<'_>, dom: &VirtualDom) -> Option<ElementId> {
        let roots: Vec<_> = vnode.vnode().template.root_slots().collect();
        for (root_idx, _static_op, dynamic_anchor) in roots {
            if let Some(anchor) = dynamic_anchor {
                for value_idx in vnode.vnode().dynamic_node_indices_for_anchor(anchor) {
                    if let Some(id) = from_dynamic(vnode, value_idx, dom) {
                        return Some(id);
                    }
                }
            } else if let Some(id) = vnode.mounted_root(root_idx, dom) {
                return Some(id);
            }
        }
        None
    }

    fn from_dynamic(
        vnode: MountedVNode<'_>,
        value_idx: usize,
        dom: &VirtualDom,
    ) -> Option<ElementId> {
        match vnode.vnode().dynamic_values[value_idx]
            .as_node()
            .expect("hydration suspense node slot must point at a dynamic node")
        {
            DynamicNode::Text(_) => vnode.mounted_dynamic_node(value_idx, dom),
            DynamicNode::Component(comp) => {
                let child = comp.mounted_scope(value_idx, vnode, dom)?;
                from_vnode(child.try_mounted_root_node()?, dom)
            }
            DynamicNode::Fragment(fragment) => {
                let mounted_children = vnode.mounted_fragment_children(value_idx, dom);
                if mounted_children.len() != fragment.len() {
                    return None;
                }

                for sub in mounted_children {
                    if let Some(id) = from_vnode(sub, dom) {
                        return Some(id);
                    }
                }
                None
            }
        }
    }

    from_vnode(scope.try_mounted_root_node()?, dom)
}

impl WebsysDom {
    /// Walk a scope's rendered VDOM, recording any nested suspense boundaries
    /// in `suspense_hydration_ids`. Used by the empty-chunk hydration path
    /// where no real DOM exists to drive the full walker, but nested
    /// streaming-suspense scopes still need their discovery-order ids
    /// registered so subsequent chunks can resolve them.
    pub(super) fn collect_suspense_only(&mut self, scope: &ScopeState, dom: &VirtualDom) {
        self.track_suspense_for_scope(scope, dom);
        if let Some(root) = scope.try_mounted_root_node() {
            self.collect_suspense_in_vnode(root, dom);
        }
    }

    fn collect_suspense_in_vnode(&mut self, vnode: MountedVNode<'_>, dom: &VirtualDom) {
        for (idx, value) in vnode.vnode().dynamic_values.iter().enumerate() {
            let Some(node) = value.as_node() else {
                continue;
            };

            match node {
                DynamicNode::Component(comp) => {
                    if let Some(child_scope) = comp.mounted_scope(idx, vnode, dom) {
                        self.collect_suspense_only(child_scope, dom);
                    }
                }
                DynamicNode::Fragment(fragment) => {
                    let mounted_children = vnode.mounted_fragment_children(idx, dom);
                    if mounted_children.len() != fragment.len() {
                        continue;
                    }

                    for sub in mounted_children {
                        self.collect_suspense_in_vnode(sub, dom);
                    }
                }
                _ => {}
            }
        }
    }

    fn track_suspense_for_scope(&mut self, scope: &ScopeState, dom: &VirtualDom) {
        if let Some(suspense) =
            SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime(), scope.id())
            && suspense.has_suspended_tasks()
        {
            self.suspense_hydration_ids
                .add_suspense_boundary(scope.id());
        }
    }
}

pub(super) fn path_to_resolved_suspense_id(path: &[u32]) -> String {
    let mut out = String::from("ds-");
    let mut iter = path.iter();
    if let Some(first) = iter.next() {
        write!(out, "{first}").unwrap();
    }
    for id in iter {
        write!(out, ",{id}").unwrap();
    }
    out.push_str("-r");
    out
}
