use crate::{
    factory::RenderReturn, nodes::VNode, virtual_dom::VirtualDom, AttributeValue, DynamicNode,
    ScopeId,
};
use bumpalo::boxed::Box as BumpBox;

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
        if el.0 == 0 {
            panic!(
                "Invalid element set to 0 - {:#?}",
                std::backtrace::Backtrace::force_capture()
            )
        }

        println!("reclaiming {:?}", el);
        self.elements.try_remove(el.0)
    }

    pub(crate) fn update_template(&mut self, el: ElementId, node: &VNode) {
        let node: *const VNode = node as *const _;
        self.elements[el.0].template = unsafe { std::mem::transmute(node) };
    }

    // Drop a scope and all its children
    pub(crate) fn drop_scope(&mut self, id: ScopeId) {
        let scope = self.scopes.get(id.0).unwrap();

        if let Some(root) = scope.as_ref().try_root_node() {
            let root = unsafe { root.extend_lifetime_ref() };
            if let RenderReturn::Sync(Ok(node)) = root {
                self.drop_scope_inner(node)
            }
        }

        let scope = self.scopes.get_mut(id.0).unwrap();
        scope.props.take();

        // Drop all the hooks once the children are dropped
        // this means we'll drop hooks bottom-up
        for hook in scope.hook_list.get_mut().drain(..) {
            println!("dropping hook !");
            drop(unsafe { BumpBox::from_raw(hook) });
            println!("hook dropped !");
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
                Some(DynamicNode::Fragment(nodes)) => {
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

impl PartialEq<&[u8]> for ElementPath {
    fn eq(&self, other: &&[u8]) -> bool {
        match *self {
            ElementPath::Deep(deep) => deep.eq(*other),
            ElementPath::Root(r) => other.len() == 1 && other[0] == r as u8,
        }
    }
}
