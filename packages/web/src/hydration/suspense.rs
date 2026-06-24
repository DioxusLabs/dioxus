use crate::dom::WebsysDom;
use dioxus_core::{DynamicNode, MountedVNode, ScopeId, ScopeState, SuspenseContext, VirtualDom};
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
        let (first, rest) = path.split_first()?;
        let root = self.children.get(*first as usize)?;
        root.traverse(rest).map(|node| node.scope_id)
    }
}

impl WebsysDom {
    /// Record suspense boundaries from the tree currently rendered in the DOM.
    /// During initial hydration this matches the server's first streaming pass:
    /// suspended boundaries reserve ids for their fallback output, but retained
    /// primary branches are not discovered until that boundary resolves.
    pub(super) fn collect_initial_suspense(&mut self, scope: &ScopeState, dom: &VirtualDom) {
        self.collect_suspense(scope, dom, false);
    }

    /// Walk a scope's rendered VDOM, recording any nested suspense boundaries
    /// in `suspense_hydration_ids`. Used by the empty-chunk hydration path
    /// where no real DOM exists to drive the full walker, but nested
    /// streaming-suspense scopes still need their discovery-order ids
    /// registered so subsequent chunks can resolve them.
    pub(super) fn collect_suspense_only(&mut self, scope: &ScopeState, dom: &VirtualDom) {
        self.collect_suspense(scope, dom, true);
    }

    fn collect_suspense(
        &mut self,
        scope: &ScopeState,
        dom: &VirtualDom,
        include_retained_branches: bool,
    ) {
        self.track_suspense_for_scope(scope, dom);

        if include_retained_branches {
            if let Some(suspense) =
                SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime(), scope.id())
            {
                let _ = suspense.with_suspended_mounted_root(|root| {
                    self.collect_suspense_in_vnode(root, dom, include_retained_branches);
                });
            }
        }

        if let Some(root) = scope.try_mounted_root_node() {
            self.collect_suspense_in_vnode(root, dom, include_retained_branches);
        }
    }

    fn collect_suspense_in_vnode(
        &mut self,
        vnode: MountedVNode<'_>,
        dom: &VirtualDom,
        include_retained_branches: bool,
    ) {
        for anchor in vnode.vnode().dynamic_anchors() {
            for slot in anchor.nodes() {
                match &*slot {
                    DynamicNode::Component(comp) => {
                        if let Some(child_scope) = comp.mounted_scope(slot, vnode, dom) {
                            self.collect_suspense(child_scope, dom, include_retained_branches);
                        }
                    }
                    DynamicNode::Fragment(fragment) => {
                        let mounted_children = vnode.mounted_fragment_children(slot, dom);
                        if mounted_children.len() != fragment.len() {
                            continue;
                        }

                        for sub in mounted_children {
                            self.collect_suspense_in_vnode(sub, dom, include_retained_branches);
                        }
                    }
                    DynamicNode::Text(_) => {}
                }
            }
        }
    }

    fn track_suspense_for_scope(&mut self, scope: &ScopeState, dom: &VirtualDom) {
        if let Some(suspense) =
            SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime(), scope.id())
        {
            if !suspense.is_suspended() || !suspense.has_suspended_tasks() {
                return;
            }
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
