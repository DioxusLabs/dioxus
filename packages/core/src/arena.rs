use crate::{
    factory::RenderReturn, nodes::VNode, virtual_dom::VirtualDom, AttributeValue, DynamicNode,
    ScopeId, VFragment,
};
use bumpalo::boxed::Box as BumpBox;

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ElementId(pub usize);

pub(crate) struct ElementRef {
    // the pathway of the real element inside the template
    pub path: ElementPath,

    // The actual template
    pub template: *mut VNode<'static>,
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

impl<'b> VirtualDom {
    pub(crate) fn next_element(&mut self, template: &VNode, path: &'static [u8]) -> ElementId {
        let entry = self.elements.vacant_entry();
        let id = entry.key();
        entry.insert(ElementRef {
            template: template as *const _ as *mut _,
            path: ElementPath::Deep(path),
        });
        ElementId(id)
    }

    pub(crate) fn next_root(&mut self, template: &VNode, path: usize) -> ElementId {
        let entry = self.elements.vacant_entry();
        let id = entry.key();
        entry.insert(ElementRef {
            template: template as *const _ as *mut _,
            path: ElementPath::Root(path),
        });
        ElementId(id)
    }

    pub(crate) fn reclaim(&mut self, el: ElementId) {
        self.try_reclaim(el)
            .unwrap_or_else(|| panic!("cannot reclaim {:?}", el));
    }

    pub(crate) fn try_reclaim(&mut self, el: ElementId) -> Option<ElementRef> {
        assert_ne!(el, ElementId(0));
        self.elements.try_remove(el.0)
    }

    // Drop a scope and all its children
    pub(crate) fn drop_scope(&mut self, id: ScopeId) {
        let scope = self.scopes.get(id.0).unwrap();

        if let Some(root) = scope.as_ref().try_root_node() {
            let root = unsafe { root.extend_lifetime_ref() };
            match root {
                RenderReturn::Sync(Ok(node)) => self.drop_scope_inner(node),
                _ => {}
            }
        }

        let scope = self.scopes.get(id.0).unwrap();

        if let Some(root) = unsafe { scope.as_ref().previous_frame().try_load_node() } {
            let root = unsafe { root.extend_lifetime_ref() };
            match root {
                RenderReturn::Sync(Ok(node)) => self.drop_scope_inner(node),
                _ => {}
            }
        }

        let scope = self.scopes.get(id.0).unwrap().as_ref();

        // Drop all the hooks once the children are dropped
        // this means we'll drop hooks bottom-up
        for hook in scope.hook_list.borrow_mut().drain(..) {
            drop(unsafe { BumpBox::from_raw(hook) });
        }
    }

    fn drop_scope_inner(&mut self, node: &VNode) {
        for attr in node.dynamic_attrs {
            if let AttributeValue::Listener(l) = &attr.value {
                l.borrow_mut().take();
            }
        }

        for (idx, _) in node.template.roots.iter().enumerate() {
            match node.dynamic_root(idx) {
                Some(DynamicNode::Component(c)) => self.drop_scope(c.scope.get().unwrap()),
                Some(DynamicNode::Fragment(VFragment::NonEmpty(nodes))) => {
                    for node in *nodes {
                        self.drop_scope_inner(node);
                    }
                }
                _ => {}
            }
        }
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

#[test]
fn path_ascendant() {
    // assert!(&ElementPath::Deep(&[]).is_ascendant(&&[0_u8]));
    // assert!(&ElementPath::Deep(&[1, 2]), &[1, 2, 3]);
    // assert!(!is_path_ascendant(
    //     &ElementPath::Deep(&[1, 2, 3, 4]),
    //     &[1, 2, 3]
    // ));
}

impl PartialEq<&[u8]> for ElementPath {
    fn eq(&self, other: &&[u8]) -> bool {
        match *self {
            ElementPath::Deep(deep) => deep.eq(*other),
            ElementPath::Root(r) => other.len() == 1 && other[0] == r as u8,
        }
    }
}
