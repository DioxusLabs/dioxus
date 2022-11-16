use crate::{nodes::VNode, virtual_dom::VirtualDom};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ElementId(pub usize);

pub struct ElementPath {
    pub template: *mut VNode<'static>,
    pub element: usize,
}

impl ElementPath {
    pub fn null() -> Self {
        Self {
            template: std::ptr::null_mut(),
            element: 0,
        }
    }
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
