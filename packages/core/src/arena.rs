use std::ptr::NonNull;

use crate::{
    innerlude::DirtyScope, nodes::RenderReturn, nodes::VNode, virtual_dom::VirtualDom,
    AttributeValue, DynamicNode, ScopeId,
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

#[derive(Debug, Clone, Copy)]
pub struct ElementRef {
    // the pathway of the real element inside the template
    pub(crate) path: ElementPath,

    // The actual template
    pub(crate) template: VNodeId,

    // The scope the element belongs to
    pub(crate) scope: ScopeId,
}

#[derive(Clone, Copy, Debug)]
pub struct ElementPath {
    pub(crate) path: &'static [u8],
}

impl VirtualDom {
    pub(crate) fn next_element(&mut self) -> ElementId {
        ElementId(self.elements.insert(None))
    }

    pub(crate) fn next_vnode_ref(&mut self, vnode: &VNode) -> VNodeId {
        let new_id = VNodeId(self.element_refs.insert(Some(unsafe {
            std::mem::transmute::<NonNull<VNode>, _>(vnode.into())
        })));

        // Set this id to be dropped when the scope is rerun
        if let Some(scope) = self.runtime.current_scope_id() {
            self.scopes[scope.0]
                .element_refs_to_drop
                .borrow_mut()
                .push(new_id);
        }

        new_id
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

    pub(crate) fn set_template(&mut self, id: VNodeId, vnode: &VNode) {
        self.element_refs[id.0] =
            Some(unsafe { std::mem::transmute::<NonNull<VNode>, _>(vnode.into()) });
    }

    // Drop a scope and all its children
    //
    // Note: This will not remove any ids from the arena
    pub(crate) fn drop_scope(&mut self, id: ScopeId, recursive: bool) {
        self.dirty_scopes.remove(&DirtyScope {
            height: self.scopes[id.0].height(),
            id,
        });

        // Remove all VNode ids from the scope
        for id in self.scopes[id.0]
            .element_refs_to_drop
            .borrow_mut()
            .drain(..)
        {
            self.element_refs.try_remove(id.0);
        }

        self.ensure_drop_safety(id);

        if recursive {
            if let Some(root) = self.scopes[id.0].try_root_node() {
                if let RenderReturn::Ready(node) = unsafe { root.extend_lifetime_ref() } {
                    self.drop_scope_inner(node)
                }
            }
        }

        let scope = &mut self.scopes[id.0];

        // Drop all the hooks once the children are dropped
        // this means we'll drop hooks bottom-up
        scope.hooks.get_mut().clear();
        {
            let context = scope.context();

            // Drop all the futures once the hooks are dropped
            for task_id in context.spawned_tasks.borrow_mut().drain() {
                context.tasks.remove(task_id);
            }
        }

        self.scopes.remove(id.0);
    }

    fn drop_scope_inner(&mut self, node: &VNode) {
        node.dynamic_nodes.iter().for_each(|node| match node {
            DynamicNode::Component(c) => {
                if let Some(f) = c.scope.get() {
                    self.drop_scope(f, true);
                }
                c.props.take();
            }
            DynamicNode::Fragment(nodes) => {
                nodes.iter().for_each(|node| self.drop_scope_inner(node))
            }
            DynamicNode::Placeholder(_) => {}
            DynamicNode::Text(_) => {}
        });
    }

    /// Descend through the tree, removing any borrowed props and listeners
    pub(crate) fn ensure_drop_safety(&mut self, scope_id: ScopeId) {
        let scope = &self.scopes[scope_id.0];

        {
            // Drop all element refs that could be invalidated when the component was rerun
            let mut element_refs = self.scopes[scope_id.0].element_refs_to_drop.borrow_mut();
            let element_refs_slab = &mut self.element_refs;
            for element_ref in element_refs.drain(..) {
                if let Some(element_ref) = element_refs_slab.get_mut(element_ref.0) {
                    *element_ref = None;
                }
            }
        }

        // make sure we drop all borrowed props manually to guarantee that their drop implementation is called before we
        // run the hooks (which hold an &mut Reference)
        // recursively call ensure_drop_safety on all children
        let props = { scope.borrowed_props.borrow_mut().clone() };
        for comp in props {
            let comp = unsafe { &*comp };
            match comp.scope.get() {
                Some(child) if child != scope_id => self.ensure_drop_safety(child),
                _ => (),
            }
            if let Ok(mut props) = comp.props.try_borrow_mut() {
                *props = None;
            }
        }
        let scope = &self.scopes[scope_id.0];
        scope.borrowed_props.borrow_mut().clear();

        // Now that all the references are gone, we can safely drop our own references in our listeners.
        let mut listeners = scope.attributes_to_drop_before_render.borrow_mut();
        listeners.drain(..).for_each(|listener| {
            let listener = unsafe { &*listener };
            if let AttributeValue::Listener(l) = &listener.value {
                _ = l.take();
            }
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
