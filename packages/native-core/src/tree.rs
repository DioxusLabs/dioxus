use crate::NodeId;
use shipyard::{Component, EntitiesViewMut, Get, View, ViewMut};
use std::fmt::Debug;

#[derive(PartialEq, Eq, Clone, Debug, Component)]
pub struct Node {
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    height: u16,
}

pub type TreeRefView<'a> = View<'a, Node>;
pub type TreeMutView<'a> = (EntitiesViewMut<'a>, ViewMut<'a, Node>);

pub trait TreeRef {
    fn parent_id(&self, id: NodeId) -> Option<NodeId>;
    fn children_ids(&self, id: NodeId) -> Option<Vec<NodeId>>;
    fn height(&self, id: NodeId) -> Option<u16>;
    fn contains(&self, id: NodeId) -> bool;
}

pub trait TreeMut: TreeRef {
    fn remove(&mut self, id: NodeId);
    fn remove_single(&mut self, id: NodeId);
    fn set_height(&mut self, node: NodeId, height: u16);
    fn create_node(&mut self, id: NodeId);
    fn add_child(&mut self, parent: NodeId, new: NodeId);
    fn replace(&mut self, old_id: NodeId, new_id: NodeId);
    fn insert_before(&mut self, old_id: NodeId, new_id: NodeId);
    fn insert_after(&mut self, old_id: NodeId, new_id: NodeId);
}

impl<'a> TreeRef for TreeRefView<'a> {
    fn parent_id(&self, id: NodeId) -> Option<NodeId> {
        self.get(id).unwrap().parent
    }

    fn children_ids(&self, id: NodeId) -> Option<Vec<NodeId>> {
        Some(self.get(id).unwrap().children.clone())
    }

    fn height(&self, id: NodeId) -> Option<u16> {
        Some(self.get(id).unwrap().height)
    }

    fn contains(&self, id: NodeId) -> bool {
        self.get(id).is_ok()
    }
}

impl<'a> TreeMut for TreeMutView<'a> {
    fn remove(&mut self, id: NodeId) {
        fn recurse<'a>(tree: &mut TreeMutView<'a>, id: NodeId) {
            let children = tree.children_ids(id);
            if let Some(children) = children {
                for child in children {
                    recurse(tree, child);
                }
            }
        }
        {
            let mut node_data_mut = &mut self.1;
            if let Some(parent) = node_data_mut.get(id).unwrap().parent {
                let parent = (&mut node_data_mut).get(parent).unwrap();
                parent.children.retain(|&child| child != id);
            }
        }

        recurse(self, id);
    }

    fn remove_single(&mut self, id: NodeId) {
        {
            let mut node_data_mut = &mut self.1;
            if let Some(parent) = node_data_mut.get(id).unwrap().parent {
                let parent = (&mut node_data_mut).get(parent).unwrap();
                parent.children.retain(|&child| child != id);
            }
        }
    }

    fn set_height(&mut self, node: NodeId, height: u16) {
        let children = {
            let mut node_data_mut = &mut self.1;
            let mut node = (&mut node_data_mut).get(node).unwrap();
            node.height = height;
            node.children.clone()
        };
        for child in children {
            self.set_height(child, height + 1);
        }
    }

    fn create_node(&mut self, id: NodeId) {
        let (entities, node_data_mut) = self;
        entities.add_component(
            id,
            node_data_mut,
            Node {
                parent: None,
                children: Vec::new(),
                height: 0,
            },
        );
    }

    fn add_child(&mut self, parent: NodeId, new: NodeId) {
        let height;
        {
            let mut node_state = &mut self.1;
            (&mut node_state).get(new).unwrap().parent = Some(parent);
            let parent = (&mut node_state).get(parent).unwrap();
            parent.children.push(new);
            height = parent.height + 1;
        }
        self.set_height(new, height);
    }

    fn replace(&mut self, old_id: NodeId, new_id: NodeId) {
        {
            let mut node_state = &mut self.1;
            // update the parent's link to the child
            if let Some(parent_id) = node_state.get(old_id).unwrap().parent {
                let parent = (&mut node_state).get(parent_id).unwrap();
                for id in &mut parent.children {
                    if *id == old_id {
                        *id = new_id;
                        break;
                    }
                }
                let height = parent.height + 1;
                self.set_height(new_id, height);
            }
        }
        // remove the old node
        self.remove(old_id);
    }

    fn insert_before(&mut self, old_id: NodeId, new_id: NodeId) {
        let mut node_state = &mut self.1;
        let old_node = node_state.get(old_id).unwrap();
        let parent_id = old_node.parent.expect("tried to insert before root");
        (&mut node_state).get(new_id).unwrap().parent = Some(parent_id);
        let parent = (&mut node_state).get(parent_id).unwrap();
        let index = parent
            .children
            .iter()
            .position(|child| *child == old_id)
            .unwrap();
        parent.children.insert(index, new_id);
        let height = parent.height + 1;
        self.set_height(new_id, height);
    }

    fn insert_after(&mut self, old_id: NodeId, new_id: NodeId) {
        let mut node_state = &mut self.1;
        let old_node = node_state.get(old_id).unwrap();
        let parent_id = old_node.parent.expect("tried to insert before root");
        (&mut node_state).get(new_id).unwrap().parent = Some(parent_id);
        let parent = (&mut node_state).get(parent_id).unwrap();
        let index = parent
            .children
            .iter()
            .position(|child| *child == old_id)
            .unwrap();
        parent.children.insert(index + 1, new_id);
        let height = parent.height + 1;
        self.set_height(new_id, height);
    }
}

impl<'a> TreeRef for TreeMutView<'a> {
    fn parent_id(&self, id: NodeId) -> Option<NodeId> {
        let node_data = &self.1;
        node_data.get(id).unwrap().parent
    }

    fn children_ids(&self, id: NodeId) -> Option<Vec<NodeId>> {
        let node_data = &self.1;
        node_data.get(id).map(|node| node.children.clone()).ok()
    }

    fn height(&self, id: NodeId) -> Option<u16> {
        let node_data = &self.1;
        node_data.get(id).map(|node| node.height).ok()
    }

    fn contains(&self, id: NodeId) -> bool {
        self.1.get(id).is_ok()
    }
}

#[test]
fn creation() {
    let mut tree = Tree::new();
    let parent_id = tree.root;
    tree.insert(parent_id, 1i32);
    let mut child = tree.create_node();
    child.insert(0i32);
    let child_id = child.id();

    tree.add_child(parent_id, child_id);

    println!("Tree: {tree:#?}");
    assert_eq!(tree.size(), 2);
    assert_eq!(tree.height(parent_id), Some(0));
    assert_eq!(tree.height(child_id), Some(1));
    assert_eq!(tree.parent_id(parent_id), None);
    assert_eq!(tree.parent_id(child_id).unwrap(), parent_id);
    assert_eq!(tree.children_ids(parent_id).unwrap(), &[child_id]);

    assert_eq!(*tree.get::<i32>(parent_id).unwrap(), 1);
    assert_eq!(*tree.get::<i32>(child_id).unwrap(), 0);
}

#[test]
fn insertion() {
    let mut tree = Tree::new();
    let parent = tree.root();
    tree.insert(parent, 0);
    let mut child = tree.create_node();
    child.insert(2);
    let child = child.id();
    tree.add_child(parent, child);
    let mut before = tree.create_node();
    before.insert(1);
    let before = before.id();
    tree.insert_before(child, before);
    let mut after = tree.create_node();
    after.insert(3);
    let after = after.id();
    tree.insert_after(child, after);

    println!("Tree: {tree:#?}");
    assert_eq!(tree.size(), 4);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(child), Some(1));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(child).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[before, child, after]);

    assert_eq!(*tree.get::<i32>(parent).unwrap(), 0);
    assert_eq!(*tree.get::<i32>(before).unwrap(), 1);
    assert_eq!(*tree.get::<i32>(child).unwrap(), 2);
    assert_eq!(*tree.get::<i32>(after).unwrap(), 3);
}

#[test]
fn deletion() {
    let mut tree = Tree::new();
    let parent = tree.root();
    tree.insert(parent, 0);
    let mut child = tree.create_node();
    child.insert(2);
    let child = child.id();
    tree.add_child(parent, child);
    let mut before = tree.create_node();
    before.insert(1);
    let before = before.id();
    tree.insert_before(child, before);
    let mut after = tree.create_node();
    after.insert(3);
    let after = after.id();
    tree.insert_after(child, after);

    println!("Tree: {tree:#?}");
    assert_eq!(tree.size(), 4);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(child), Some(1));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(child).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[before, child, after]);

    assert_eq!(*tree.get::<i32>(parent).unwrap(), 0);
    assert_eq!(*tree.get::<i32>(before).unwrap(), 1);
    assert_eq!(*tree.get::<i32>(child).unwrap(), 2);
    assert_eq!(*tree.get::<i32>(after).unwrap(), 3);

    tree.remove(child);

    println!("Tree: {tree:#?}");
    assert_eq!(tree.size(), 3);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[before, after]);

    assert_eq!(*tree.get::<i32>(parent).unwrap(), 0);
    assert_eq!(*tree.get::<i32>(before).unwrap(), 1);
    assert_eq!(tree.get::<i32>(child), None);
    assert_eq!(*tree.get::<i32>(after).unwrap(), 3);

    tree.remove(before);

    println!("Tree: {tree:#?}");
    assert_eq!(tree.size(), 2);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[after]);

    assert_eq!(*tree.get::<i32>(parent).unwrap(), 0);
    assert_eq!(tree.get::<i32>(before), None);
    assert_eq!(*tree.get::<i32>(after).unwrap(), 3);

    tree.remove(after);

    println!("Tree: {tree:#?}");
    assert_eq!(tree.size(), 1);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.children_ids(parent).unwrap(), &[]);

    assert_eq!(*tree.get::<i32>(parent).unwrap(), 0);
    assert_eq!(tree.get::<i32>(after), None);
}
