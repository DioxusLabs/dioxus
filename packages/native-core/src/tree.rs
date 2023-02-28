use crate::NodeId;
use shipyard::{Component, EntitiesView, EntityId, Get, Unique, View, ViewMut, World};
use std::fmt::Debug;

#[derive(PartialEq, Eq, Clone, Debug, Component)]
pub(crate) struct Node {
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    height: u16,
}

#[derive(Debug)]
pub(crate) struct Tree {
    pub(crate) nodes: World,
    root: NodeId,
}

impl Tree {
    pub fn new() -> Self {
        let mut nodes = World::default();
        let node = Node {
            parent: None,
            children: Vec::new(),
            height: 0,
        };
        let root = nodes.add_entity((node,));
        Self { nodes, root }
    }

    pub fn add_unique<T: Unique + Send + Sync>(&mut self, data: T) {
        self.nodes.add_unique(data);
    }

    pub fn remove_unique<T: Unique + Send + Sync>(
        &mut self,
    ) -> Result<T, shipyard::error::UniqueRemove> {
        self.nodes.remove_unique::<T>()
    }

    fn node_data(&self) -> View<Node> {
        self.nodes.borrow().unwrap()
    }

    fn node_data_mut(&self) -> ViewMut<Node> {
        self.nodes.borrow().unwrap()
    }

    pub fn remove(&mut self, id: NodeId) {
        fn recurse(tree: &mut Tree, id: NodeId) {
            let children = tree.children_ids(id);
            if let Some(children) = children {
                for child in children {
                    recurse(tree, child);
                }
            }

            tree.nodes.delete_entity(id);
        }
        {
            let mut node_data_mut = self.node_data_mut();
            if let Some(parent) = node_data_mut.get(id).unwrap().parent {
                let parent = (&mut node_data_mut).get(parent).unwrap();
                parent.children.retain(|&child| child != id);
            }
        }

        recurse(self, id);
    }

    pub fn remove_single(&mut self, id: NodeId) {
        {
            let mut node_data_mut = self.node_data_mut();
            if let Some(parent) = node_data_mut.get(id).unwrap().parent {
                let parent = (&mut node_data_mut).get(parent).unwrap();
                parent.children.retain(|&child| child != id);
            }
        }

        self.nodes.delete_entity(id);
    }

    fn set_height(&mut self, node: NodeId, height: u16) {
        let children = {
            let mut node_data_mut = self.node_data_mut();
            let mut node = (&mut node_data_mut).get(node).unwrap();
            node.height = height;
            node.children.clone()
        };
        for child in children {
            self.set_height(child, height + 1);
        }
    }

    pub fn create_node(&mut self) -> NodeBuilder<'_> {
        let node = self.nodes.add_entity((Node {
            parent: None,
            children: Vec::new(),
            height: 0,
        },));
        NodeBuilder { tree: self, node }
    }

    pub fn add_child(&mut self, parent: NodeId, new: NodeId) {
        let height;
        {
            let mut node_state = self.node_data_mut();
            (&mut node_state).get(new).unwrap().parent = Some(parent);
            let parent = (&mut node_state).get(parent).unwrap();
            parent.children.push(new);
            height = parent.height + 1;
        }
        self.set_height(new, height);
    }

    pub fn replace(&mut self, old_id: NodeId, new_id: NodeId) {
        {
            let mut node_state = self.node_data_mut();
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
                drop(node_state);
                self.set_height(new_id, height);
            }
        }
        // remove the old node
        self.remove(old_id);
    }

    pub fn insert_before(&mut self, old_id: NodeId, new_id: NodeId) {
        let mut node_state = self.node_data_mut();
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
        drop(node_state);
        self.set_height(new_id, height);
    }

    pub fn insert_after(&mut self, old_id: NodeId, new_id: NodeId) {
        let mut node_state = self.node_data_mut();
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
        drop(node_state);
        self.set_height(new_id, height);
    }

    pub fn insert<T: Component + Sync + Send>(&mut self, id: NodeId, value: T) {
        self.nodes.add_component(id, value);
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.borrow::<EntitiesView>().unwrap().is_alive(id)
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn parent_id(&self, id: NodeId) -> Option<NodeId> {
        let node_data = self.node_data();
        node_data.get(id).unwrap().parent
    }

    pub fn children_ids(&self, id: NodeId) -> Option<Vec<NodeId>> {
        let node_data = self.node_data();
        node_data.get(id).map(|node| node.children.clone()).ok()
    }

    pub fn height(&self, id: NodeId) -> Option<u16> {
        let node_data = self.node_data();
        node_data.get(id).map(|node| node.height).ok()
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

pub struct NodeBuilder<'a> {
    tree: &'a mut Tree,
    node: EntityId,
}

impl<'a> NodeBuilder<'a> {
    pub fn insert<T: Component + Send + Sync>(&mut self, component: T) {
        self.tree.insert(self.node, component);
    }

    pub fn id(&self) -> EntityId {
        self.node
    }
}
