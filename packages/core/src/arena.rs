use crate::{
    nodes::VNode, virtual_dom::VirtualDom, DynamicNode,
    ScopeId,
};

/// An Element's unique identifier.
///
/// `ElementId` is a `usize` that is unique across the entire VirtualDOM - but not unique across time. If a component is
/// unmounted, then the `ElementId` will be reused for a new component.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ElementId(pub usize);

/// An Element that can be bubbled to's unique identifier.
///
/// `BubbleId` is a `usize` that is unique across the entire VirtualDOM - but not unique across time. If a component is
/// unmounted, then the `BubbleId` will be reused for a new component.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct VNodeId(pub usize);

#[derive(Debug, Clone)]
pub struct ElementRef {
    // the pathway of the real element inside the template
    pub(crate) path: ElementPath,

    // the scope that this element belongs to
    pub(crate) scope: ScopeId,

    // The actual element
    pub(crate) element: VNode,
}

#[derive(Clone, Copy, Debug)]
pub struct ElementPath {
    pub(crate) path: &'static [u8],
}

impl VirtualDom {
    pub(crate) fn next_element(&mut self) -> ElementId {
        ElementId(self.elements.insert(None))
    }

    pub(crate) fn reclaim(&mut self, el: ElementId) {
        self.try_reclaim(el)
            .unwrap_or_else(|| panic!("cannot reclaim {:?}", el));
    }

    pub(crate) fn try_reclaim(&mut self, el: ElementId) -> Option<()> {
        if el.0 == 0 {
            panic!(
                "Cannot reclaim the root element - {:#?}",
                std::backtrace::Backtrace::force_capture()
            );
        }

        self.elements.try_remove(el.0).map(|_| ())
    }

    // Drop a scope and all its children
    //
    // Note: This will not remove any ids from the arena
    pub(crate) fn drop_scope(&mut self, _id: ScopeId, _recursive: bool) {
        // todo: Do we need this now that we don't have a bunch of unsafe code?
        // self.dirty_scopes.remove(&DirtyScope {
        //     height: self.scopes[id.0].height(),
        //     id,
        // });

        // if recursive {
        //     if let Some(root) = self.scopes[id.0].try_root_node() {
        //         if let RenderReturn::Ready(node) = root {
        //             self.drop_scope_inner(node)
        //         }
        //     }
        // }

        // self.scopes.remove(id.0);
    }

    fn drop_scope_inner(&mut self, node: &VNode) {
        node.dynamic_nodes.iter().for_each(|node| match node {
            DynamicNode::Component(c) => {
                if let Some(f) = c.scope.get() {
                    self.drop_scope(f, true);
                }
            }
            DynamicNode::Fragment(nodes) => {
                nodes.iter().for_each(|node| self.drop_scope_inner(node))
            }
            DynamicNode::Placeholder(_) => {}
            DynamicNode::Text(_) => {}
        });
    }
}

impl ElementPath {
    pub(crate) fn is_decendant(&self, small: &&[u8]) -> bool {
        small.len() <= self.path.len() && *small == &self.path[..small.len()]
    }
}

impl PartialEq<&[u8]> for ElementPath {
    fn eq(&self, other: &&[u8]) -> bool {
        self.path.eq(*other)
    }
}
