//! A tree of nodes intigated with shipyard

use crate::NodeId;
use shipyard::{Component, EntitiesViewMut, Get, View, ViewMut};
use std::fmt::Debug;

/// A node in a tree.
#[derive(PartialEq, Eq, Clone, Debug, Component)]
pub struct Node {
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    height: u16,
}

/// A view of a tree.
pub type TreeRefView<'a> = View<'a, Node>;
/// A mutable view of a tree.
pub type TreeMutView<'a> = (EntitiesViewMut<'a>, ViewMut<'a, Node>);

/// A immutable view of a tree.
pub trait TreeRef {
    /// The parent id of the node.
    fn parent_id(&self, id: NodeId) -> Option<NodeId>;
    /// The children ids of the node.
    fn children_ids(&self, id: NodeId) -> Vec<NodeId>;
    /// The height of the node.
    fn height(&self, id: NodeId) -> Option<u16>;
    /// Returns true if the node exists.
    fn contains(&self, id: NodeId) -> bool;
}

/// A mutable view of a tree.
pub trait TreeMut: TreeRef {
    /// Removes the node and all of its children.
    fn remove(&mut self, id: NodeId);
    /// Removes the node and all of its children.
    fn remove_single(&mut self, id: NodeId);
    /// Adds a new node to the tree.
    fn create_node(&mut self, id: NodeId);
    /// Adds a child to the node.
    fn add_child(&mut self, parent: NodeId, new: NodeId);
    /// Replaces the node with a new node.
    fn replace(&mut self, old_id: NodeId, new_id: NodeId);
    /// Inserts a node before another node.
    fn insert_before(&mut self, old_id: NodeId, new_id: NodeId);
    /// Inserts a node after another node.
    fn insert_after(&mut self, old_id: NodeId, new_id: NodeId);
}

impl<'a> TreeRef for TreeRefView<'a> {
    fn parent_id(&self, id: NodeId) -> Option<NodeId> {
        self.get(id).ok()?.parent
    }

    fn children_ids(&self, id: NodeId) -> Vec<NodeId> {
        self.get(id)
            .map(|node| node.children.clone())
            .unwrap_or_default()
    }

    fn height(&self, id: NodeId) -> Option<u16> {
        Some(self.get(id).ok()?.height)
    }

    fn contains(&self, id: NodeId) -> bool {
        self.get(id).is_ok()
    }
}

impl<'a> TreeMut for TreeMutView<'a> {
    fn remove(&mut self, id: NodeId) {
        fn recurse(tree: &mut TreeMutView<'_>, id: NodeId) {
            let children = tree.children_ids(id);
            for child in children {
                recurse(tree, child);
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
        set_height(self, new, height);
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
                set_height(self, new_id, height);
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
        set_height(self, new_id, height);
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
        set_height(self, new_id, height);
    }
}

/// Sets the height of a node and updates the height of all its children
fn set_height(tree: &mut TreeMutView<'_>, node: NodeId, height: u16) {
    let children = {
        let mut node_data_mut = &mut tree.1;
        let mut node = (&mut node_data_mut).get(node).unwrap();
        node.height = height;
        node.children.clone()
    };
    for child in children {
        set_height(tree, child, height + 1);
    }
}

impl<'a> TreeRef for TreeMutView<'a> {
    fn parent_id(&self, id: NodeId) -> Option<NodeId> {
        let node_data = &self.1;
        node_data.get(id).unwrap().parent
    }

    fn children_ids(&self, id: NodeId) -> Vec<NodeId> {
        let node_data = &self.1;
        node_data
            .get(id)
            .map(|node| node.children.clone())
            .unwrap_or_default()
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
    use shipyard::World;
    #[derive(Component)]
    struct Num(i32);

    let mut world = World::new();
    let parent_id = world.add_entity(Num(1i32));
    let child_id = world.add_entity(Num(0i32));

    let mut tree = world.borrow::<TreeMutView>().unwrap();

    tree.create_node(parent_id);
    tree.create_node(child_id);

    tree.add_child(parent_id, child_id);

    assert_eq!(tree.height(parent_id), Some(0));
    assert_eq!(tree.height(child_id), Some(1));
    assert_eq!(tree.parent_id(parent_id), None);
    assert_eq!(tree.parent_id(child_id).unwrap(), parent_id);
    assert_eq!(tree.children_ids(parent_id), &[child_id]);
}

#[test]
fn insertion() {
    use shipyard::World;
    #[derive(Component)]
    struct Num(i32);

    let mut world = World::new();
    let parent = world.add_entity(Num(0));
    let child = world.add_entity(Num(2));
    let before = world.add_entity(Num(1));
    let after = world.add_entity(Num(3));

    let mut tree = world.borrow::<TreeMutView>().unwrap();

    tree.create_node(parent);
    tree.create_node(child);
    tree.create_node(before);
    tree.create_node(after);

    tree.add_child(parent, child);
    tree.insert_before(child, before);
    tree.insert_after(child, after);

    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(child), Some(1));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(child).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent), &[before, child, after]);
}

#[test]
fn deletion() {
    use shipyard::World;
    #[derive(Component)]
    struct Num(i32);

    let mut world = World::new();
    let parent = world.add_entity(Num(0));
    let child = world.add_entity(Num(2));
    let before = world.add_entity(Num(1));
    let after = world.add_entity(Num(3));

    let mut tree = world.borrow::<TreeMutView>().unwrap();

    tree.create_node(parent);
    tree.create_node(child);
    tree.create_node(before);
    tree.create_node(after);

    tree.add_child(parent, child);
    tree.insert_before(child, before);
    tree.insert_after(child, after);

    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(child), Some(1));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(child).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent), &[before, child, after]);

    tree.remove(child);

    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent), &[before, after]);

    tree.remove(before);

    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent), &[after]);

    tree.remove(after);

    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.children_ids(parent), &[]);
}
