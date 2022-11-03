use crate::{nodes::VNode, virtualdom::VirtualDom};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ElementId(pub usize);

pub struct ElementPath {
    pub template: *mut VNode<'static>,
    pub element: usize,
}

impl VirtualDom {
    pub fn next_element(&mut self, template: &VNode) -> ElementId {
        let entry = self.elements.vacant_entry();
        let id = entry.key();

        entry.insert(ElementPath {
            template: template as *const _ as *mut _,
            element: id,
        });

        ElementId(id)
    }

    pub fn cleanup_element(&mut self, id: ElementId) {
        self.elements.remove(id.0);
    }
}
