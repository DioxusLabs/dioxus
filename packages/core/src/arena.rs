use crate::{nodes::VNode, virtual_dom::VirtualDom, Mutations, ScopeId};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ElementId(pub usize);

pub struct ElementRef {
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
    pub fn null() -> Self {
        Self {
            template: std::ptr::null_mut(),
            path: ElementPath::Root(0),
        }
    }
}

impl<'b> VirtualDom {
    pub fn next_element(&mut self, template: &VNode, path: &'static [u8]) -> ElementId {
        let entry = self.elements.vacant_entry();
        let id = entry.key();

        entry.insert(ElementRef {
            template: template as *const _ as *mut _,
            path: ElementPath::Deep(path),
        });

        println!("Claiming {}", id);

        ElementId(id)
    }

    pub fn next_root(&mut self, template: &VNode, path: usize) -> ElementId {
        let entry = self.elements.vacant_entry();
        let id = entry.key();

        entry.insert(ElementRef {
            template: template as *const _ as *mut _,
            path: ElementPath::Root(path),
        });

        println!("Claiming {}", id);

        ElementId(id)
    }

    pub fn cleanup_element(&mut self, id: ElementId) {
        self.elements.remove(id.0);
    }

    pub fn drop_scope(&mut self, id: ScopeId) {
        // let scope = self.scopes.get(id.0).unwrap();

        // let root = scope.root_node();
        // let root = unsafe { std::mem::transmute(root) };

        // self.drop_template(root, false);
        todo!()
    }

    pub fn reclaim(&mut self, el: ElementId) {
        assert_ne!(el, ElementId(0));
        self.elements.remove(el.0);
    }

    pub fn drop_template(
        &mut self,
        mutations: &mut Mutations,
        template: &'b VNode<'b>,
        gen_roots: bool,
    ) {
        // for node in template.dynamic_nodes.iter() {
        //     match node {
        //         DynamicNode::Text { id, .. } => {}

        //         DynamicNode::Component { .. } => {
        //             todo!()
        //         }

        //         DynamicNode::Fragment { inner, nodes } => {}
        //         DynamicNode::Placeholder(_) => todo!(),
        //         _ => todo!(),
        //     }
        // }
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
