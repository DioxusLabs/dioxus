use crate::{nodes::RenderReturn, nodes::VNode, virtual_dom::VirtualDom, DynamicNode, ScopeId};
use bumpalo::boxed::Box as BumpBox;

/// An Element's unique identifier.
///
/// `ElementId` is a `usize` that is unique across the entire VirtualDOM - but not unique across time. If a component is
/// unmounted, then the `ElementId` will be reused for a new component.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ElementId(pub usize);

pub(crate) struct ElementRef {
    // the pathway of the real element inside the template
    pub path: ElementPath,

    // The actual template
    pub template: *const VNode<'static>,
}

#[derive(Clone, Copy)]
pub enum ElementPath {
    Deep(&'static [u8]),
    Root(usize),
}

impl ElementRef {
    pub(crate) fn null() -> Self {
        Self {
            template: std::ptr::null_mut(),
            path: ElementPath::Root(0),
        }
    }
}

impl VirtualDom {
    pub(crate) fn next_element(&mut self, template: &VNode, path: &'static [u8]) -> ElementId {
        self.next(template, ElementPath::Deep(path))
    }

    pub(crate) fn next_root(&mut self, template: &VNode, path: usize) -> ElementId {
        self.next(template, ElementPath::Root(path))
    }

    fn next(&mut self, template: &VNode, path: ElementPath) -> ElementId {
        let entry = self.elements.vacant_entry();
        let id = entry.key();

        entry.insert(ElementRef {
            template: template as *const _ as *mut _,
            path,
        });
        ElementId(id)
    }

    pub(crate) fn reclaim(&mut self, el: ElementId) {
        self.try_reclaim(el)
            .unwrap_or_else(|| panic!("cannot reclaim {:?}", el));
    }

    pub(crate) fn try_reclaim(&mut self, el: ElementId) -> Option<ElementRef> {
        if el.0 == 0 {
            panic!(
                "Cannot reclaim the root element - {:#?}",
                std::backtrace::Backtrace::force_capture()
            );
        }

        self.elements.try_remove(el.0)
    }

    pub(crate) fn update_template(&mut self, el: ElementId, node: &VNode) {
        let node: *const VNode = node as *const _;
        self.elements[el.0].template = unsafe { std::mem::transmute(node) };
    }

    // Drop a scope and all its children
    pub(crate) fn drop_scope(&mut self, id: ScopeId) {
        if let Some(root) = self.scopes[id.0].as_ref().try_root_node() {
            if let RenderReturn::Sync(Ok(node)) = unsafe { root.extend_lifetime_ref() } {
                self.drop_scope_inner(node)
            }
        }

        self.scopes[id.0].props.take();

        let scope = &mut self.scopes[id.0];

        // Drop all the hooks once the children are dropped
        // this means we'll drop hooks bottom-up
        for hook in scope.hook_list.get_mut().drain(..) {
            drop(unsafe { BumpBox::from_raw(hook) });
        }
    }

    fn drop_scope_inner(&mut self, node: &VNode) {
        node.clear_listeners();
        node.dynamic_nodes.iter().for_each(|node| match node {
            DynamicNode::Component(c) => self.drop_scope(c.scope.get().unwrap()),
            DynamicNode::Fragment(nodes) => nodes
                .into_iter()
                .for_each(|node| self.drop_scope_inner(node)),
            DynamicNode::Placeholder(t) => {
                self.try_reclaim(t.get());
            }
            DynamicNode::Text(t) => {
                self.try_reclaim(t.id.get());
            }
        });

        for root in node.root_ids {
            let id = root.get();
            if id.0 != 0 {
                self.try_reclaim(id);
            }
        }
    }

    /// Descend through the tree, removing any borrowed props and listeners
    pub(crate) fn ensure_drop_safety(&self, scope: ScopeId) {
        let node = unsafe { self.scopes[scope.0].previous_frame().try_load_node() };

        // And now we want to make sure the previous frame has dropped anything that borrows self
        if let Some(RenderReturn::Sync(Ok(node))) = node {
            self.ensure_drop_safety_inner(node);
        }
    }

    fn ensure_drop_safety_inner(&self, node: &VNode) {
        node.clear_listeners();

        node.dynamic_nodes.iter().for_each(|child| match child {
            // Only descend if the props are borrowed
            DynamicNode::Component(c) if !c.static_props => {
                self.ensure_drop_safety(c.scope.get().unwrap());
                c.props.set(None);
            }

            DynamicNode::Fragment(f) => f
                .iter()
                .for_each(|node| self.ensure_drop_safety_inner(node)),

            _ => {}
        });
    }
}

impl ElementPath {
    pub(crate) fn is_ascendant(&self, big: &&[u8]) -> bool {
        match *self {
            ElementPath::Deep(small) => small.len() <= big.len() && small == &big[..small.len()],
            ElementPath::Root(r) => big.len() == 1 && big[0] == r as u8,
        }
    }
}

impl PartialEq<&[u8]> for ElementPath {
    fn eq(&self, other: &&[u8]) -> bool {
        match *self {
            ElementPath::Deep(deep) => deep.eq(*other),
            ElementPath::Root(r) => other.len() == 1 && other[0] == r as u8,
        }
    }
}
