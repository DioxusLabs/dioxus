use crate::{nodes::VNode, virtual_dom::VirtualDom};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ElementId(pub usize);

pub struct ElementRef {
    // the pathway of the real element inside the template
    pub path: &'static [u8],

    // The actual template
    pub template: *mut VNode<'static>,
}

impl ElementRef {
    pub fn null() -> Self {
        Self {
            template: std::ptr::null_mut(),
            path: &[],
        }
    }
}

impl VirtualDom {
    pub fn next_element(&mut self, template: &VNode, path: &'static [u8]) -> ElementId {
        let entry = self.elements.vacant_entry();
        let id = entry.key();

        entry.insert(ElementRef {
            template: template as *const _ as *mut _,
            path,
        });

        println!("Claiming {}", id);

        ElementId(id)
    }

    pub fn cleanup_element(&mut self, id: ElementId) {
        self.elements.remove(id.0);
    }
}

/*
now......

an ID is mostly a pointer to a node in the real dom.
We need to
*/
