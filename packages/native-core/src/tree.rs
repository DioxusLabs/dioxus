//! A tree of nodes intigated with shipyard

use crate::NodeId;
use shipyard::{Component, EntitiesViewMut, Get, View, ViewMut};
use std::fmt::Debug;

/// A shadow tree reference inside of a tree. This tree is isolated from the main tree.
#[derive(PartialEq, Eq, Clone, Debug, Component)]
pub struct ShadowTree {
    /// The root of the shadow tree
    pub shadow_roots: Vec<NodeId>,
    /// The node that children of the super tree should be inserted under.
    pub slot: Option<NodeId>,
}

/// A node in a tree.
#[derive(PartialEq, Eq, Clone, Debug, Component)]
pub struct Node {
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    child_subtree: Option<ShadowTree>,
    /// If this node is a slot in a shadow_tree, this is node whose child_subtree is that shadow_tree.
    slot_for_light_tree: Option<NodeId>,
    /// If this node is a root of a shadow_tree, this is the node whose child_subtree is that shadow_tree.
    root_for_light_tree: Option<NodeId>,
    height: u16,
}

/// A view of a tree.
pub type TreeRefView<'a> = View<'a, Node>;
/// A mutable view of a tree.
pub type TreeMutView<'a> = (EntitiesViewMut<'a>, ViewMut<'a, Node>);

/// A immutable view of a tree.
pub trait TreeRef {
    /// Get the id of the parent of the current node, if enter_shadow_dom is true and the current node is a shadow root, the node the shadow root is attached to will be returned
    #[inline]
    fn parent_id_advanced(&self, id: NodeId, enter_shadow_dom: bool) -> Option<NodeId> {
        // If this node is the root of a shadow_tree, return the node the shadow_tree is attached
        let root_for_light_tree = self.root_for_light_tree(id);
        match (root_for_light_tree, enter_shadow_dom) {
            (Some(id), true) => Some(id),
            _ => {
                let parent_id = self.parent_id(id);
                if enter_shadow_dom {
                    // If this node is attached via a slot, return the slot as the parent instead of the light tree parent
                    parent_id.map(|id| {
                        self.shadow_tree(id)
                            .and_then(|tree| tree.slot)
                            .unwrap_or(id)
                    })
                } else {
                    parent_id
                }
            }
        }
    }
    /// The parent id of the node.
    fn parent_id(&self, id: NodeId) -> Option<NodeId>;
    /// Get the ids of the children of the current node, if enter_shadow_dom is true and the current node is a shadow slot, the ids of the nodes under the node the shadow slot is attached to will be returned
    #[inline]
    fn children_ids_advanced(&self, id: NodeId, enter_shadow_dom: bool) -> Vec<NodeId> {
        let shadow_tree = self.shadow_tree(id);
        let slot_of_light_tree = self.slot_for_light_tree(id);
        match (shadow_tree, slot_of_light_tree, enter_shadow_dom) {
            // If this node is a shadow root, return the shadow roots
            (Some(tree), _, true) => tree.shadow_roots.clone(),
            // If this node is a slot, return the children of the node the slot is attached to
            (None, Some(id), true) => self.children_ids(id),
            _ => self.children_ids(id),
        }
    }
    /// The children ids of the node.
    fn children_ids(&self, id: NodeId) -> Vec<NodeId>;
    /// The shadow tree tree under the node.
    fn shadow_tree(&self, id: NodeId) -> Option<&ShadowTree>;
    /// The node that contains the shadow tree this node is a slot for
    fn slot_for_light_tree(&self, id: NodeId) -> Option<NodeId>;
    /// The node that contains the shadow tree this node is a root of
    fn root_for_light_tree(&self, id: NodeId) -> Option<NodeId>;
    /// The height of the node.
    fn height(&self, id: NodeId) -> Option<u16>;
    /// Returns true if the node exists.
    fn contains(&self, id: NodeId) -> bool;
}

/// A mutable view of a tree.
pub trait TreeMut: TreeRef {
    /// Removes the node and its children from the tree but do not delete the entities.
    fn remove(&mut self, id: NodeId);
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
    /// Creates a new shadow tree.
    fn create_subtree(&mut self, id: NodeId, shadow_roots: Vec<NodeId>, slot: Option<NodeId>);
    /// Remove any shadow tree.
    fn remove_subtree(&mut self, id: NodeId);
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

    fn shadow_tree(&self, id: NodeId) -> Option<&ShadowTree> {
        self.get(id).ok()?.child_subtree.as_ref()
    }

    fn slot_for_light_tree(&self, id: NodeId) -> Option<NodeId> {
        self.get(id).ok()?.slot_for_light_tree
    }

    fn root_for_light_tree(&self, id: NodeId) -> Option<NodeId> {
        self.get(id).ok()?.root_for_light_tree
    }
}

impl<'a> TreeMut for TreeMutView<'a> {
    fn remove(&mut self, id: NodeId) {
        fn recurse(tree: &mut TreeMutView<'_>, id: NodeId) {
            let (light_tree, children) = {
                let node = (&mut tree.1).get(id).unwrap();
                (node.slot_for_light_tree, std::mem::take(&mut node.children))
            };

            for child in children {
                recurse(tree, child);
            }

            // If this node is a slot in a shadow_tree, remove it from the shadow_tree.
            if let Some(light_tree) = light_tree {
                let root_for_light_tree = (&mut tree.1).get(light_tree).unwrap();

                if let Some(shadow_tree) = &mut root_for_light_tree.child_subtree {
                    shadow_tree.slot = None;
                }

                debug_assert!(
                    root_for_light_tree.children.is_empty(),
                    "ShadowTree root should have no children when slot is removed."
                );
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

    fn create_node(&mut self, id: NodeId) {
        let (entities, node_data_mut) = self;
        entities.add_component(
            id,
            node_data_mut,
            Node {
                parent: None,
                children: Vec::new(),
                height: 0,
                child_subtree: None,
                slot_for_light_tree: None,
                root_for_light_tree: None,
            },
        );
    }

    fn add_child(&mut self, parent: NodeId, new: NodeId) {
        {
            let mut node_state = &mut self.1;
            (&mut node_state).get(new).unwrap().parent = Some(parent);
            let parent = (&mut node_state).get(parent).unwrap();
            parent.children.push(new);
        }
        let height = child_height((&self.1).get(parent).unwrap(), self);
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
                let height = child_height((&self.1).get(parent_id).unwrap(), self);
                set_height(self, new_id, height);
            }
        }
        self.remove(old_id);
    }

    fn insert_before(&mut self, old_id: NodeId, new_id: NodeId) {
        let parent_id = {
            let old_node = self.1.get(old_id).unwrap();
            old_node.parent.expect("tried to insert before root")
        };
        {
            (&mut self.1).get(new_id).unwrap().parent = Some(parent_id);
        }
        let parent = (&mut self.1).get(parent_id).unwrap();
        let index = parent
            .children
            .iter()
            .position(|child| *child == old_id)
            .unwrap();
        parent.children.insert(index, new_id);
        let height = child_height((&self.1).get(parent_id).unwrap(), self);
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
        let height = child_height((&self.1).get(parent_id).unwrap(), self);
        set_height(self, new_id, height);
    }

    fn create_subtree(&mut self, id: NodeId, shadow_roots: Vec<NodeId>, slot: Option<NodeId>) {
        let (_, node_data_mut) = self;

        let light_root_height;
        {
            let shadow_tree = ShadowTree {
                shadow_roots: shadow_roots.clone(),
                slot,
            };

            let light_root = node_data_mut
                .get(id)
                .expect("tried to create shadow_tree with non-existent id");

            light_root.child_subtree = Some(shadow_tree);
            light_root_height = light_root.height;

            if let Some(slot) = slot {
                let slot = node_data_mut
                    .get(slot)
                    .expect("tried to create shadow_tree with non-existent slot");
                slot.slot_for_light_tree = Some(id);
            }
        }

        // Now that we have created the shadow_tree, we need to update the height of the shadow_tree roots
        for root in shadow_roots {
            (&mut self.1).get(root).unwrap().root_for_light_tree = Some(id);
            set_height(self, root, light_root_height + 1);
        }
    }

    fn remove_subtree(&mut self, id: NodeId) {
        let (_, node_data_mut) = self;

        if let Ok(node) = node_data_mut.get(id) {
            if let Some(shadow_tree) = node.child_subtree.take() {
                // Remove the slot's link to the shadow_tree
                if let Some(slot) = shadow_tree.slot {
                    let slot = node_data_mut
                        .get(slot)
                        .expect("tried to remove shadow_tree with non-existent slot");
                    slot.slot_for_light_tree = None;
                }

                let node = node_data_mut.get(id).unwrap();

                // Reset the height of the light root's children
                let height = node.height;
                for child in node.children.clone() {
                    println!("child: {:?}", child);
                    set_height(self, child, height + 1);
                }

                // Reset the height of the shadow roots
                for root in &shadow_tree.shadow_roots {
                    set_height(self, *root, 0);
                }
            }
        }
    }
}

fn child_height(parent: &Node, tree: &impl TreeRef) -> u16 {
    match &parent.child_subtree {
        Some(shadow_tree) => {
            if let Some(slot) = shadow_tree.slot {
                tree.height(slot)
                    .expect("Attempted to read a slot that does not exist")
                    + 1
            } else {
                panic!("Attempted to read the height of a child of a node with a shadow tree, but the shadow tree does not have a slot. Every shadow tree attached to a node with children must have a slot.")
            }
        }
        None => parent.height + 1,
    }
}

/// Sets the height of a node and updates the height of all its children
fn set_height(tree: &mut TreeMutView<'_>, node: NodeId, height: u16) {
    let (shadow_tree, light_tree, children) = {
        let mut node_data_mut = &mut tree.1;
        let node = (&mut node_data_mut).get(node).unwrap();
        node.height = height;

        (
            node.child_subtree.clone(),
            node.slot_for_light_tree,
            node.children.clone(),
        )
    };

    // If the children are actually part of a shadow_tree, there height is determined by the height of the shadow_tree
    if let Some(shadow_tree) = shadow_tree {
        // Set the height of the shadow_tree roots
        for &shadow_root in &shadow_tree.shadow_roots {
            set_height(tree, shadow_root, height + 1);
        }
    } else {
        // Otherwise, we just set the height of the children to be one more than the height of the parent
        for child in children {
            set_height(tree, child, height + 1);
        }
    }

    // If this nodes is a slot for a shadow_tree, we need to go to the super tree and update the height of its children
    if let Some(light_tree) = light_tree {
        let children = (&tree.1).get(light_tree).unwrap().children.clone();
        for child in children {
            set_height(tree, child, height + 1);
        }
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

    fn shadow_tree(&self, id: NodeId) -> Option<&ShadowTree> {
        let node_data = &self.1;
        node_data.get(id).ok()?.child_subtree.as_ref()
    }

    fn slot_for_light_tree(&self, id: NodeId) -> Option<NodeId> {
        let node_data = &self.1;
        node_data.get(id).ok()?.slot_for_light_tree
    }

    fn root_for_light_tree(&self, id: NodeId) -> Option<NodeId> {
        let node_data = &self.1;
        node_data.get(id).ok()?.root_for_light_tree
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
fn shadow_tree() {
    use shipyard::World;
    #[derive(Component)]
    struct Num(i32);

    let mut world = World::new();
    // Create main tree
    let parent_id = world.add_entity(Num(1i32));
    let child_id = world.add_entity(Num(0i32));

    // Create shadow tree
    let shadow_parent_id = world.add_entity(Num(2i32));
    let shadow_child_id = world.add_entity(Num(3i32));

    let mut tree = world.borrow::<TreeMutView>().unwrap();

    tree.create_node(parent_id);
    tree.create_node(child_id);

    tree.add_child(parent_id, child_id);

    tree.create_node(shadow_parent_id);
    tree.create_node(shadow_child_id);

    tree.add_child(shadow_parent_id, shadow_child_id);

    // Check that both trees are correct individually
    assert_eq!(tree.height(parent_id), Some(0));
    assert_eq!(tree.height(child_id), Some(1));
    assert_eq!(tree.parent_id(parent_id), None);
    assert_eq!(tree.parent_id(child_id).unwrap(), parent_id);
    assert_eq!(tree.children_ids(parent_id), &[child_id]);

    assert_eq!(tree.height(shadow_parent_id), Some(0));
    assert_eq!(tree.height(shadow_child_id), Some(1));
    assert_eq!(tree.parent_id(shadow_parent_id), None);
    assert_eq!(tree.parent_id(shadow_child_id).unwrap(), shadow_parent_id);
    assert_eq!(tree.children_ids(shadow_parent_id), &[shadow_child_id]);

    // Add shadow tree to main tree
    tree.create_subtree(parent_id, vec![shadow_parent_id], Some(shadow_child_id));

    assert_eq!(tree.height(parent_id), Some(0));
    assert_eq!(tree.height(shadow_parent_id), Some(1));
    assert_eq!(tree.height(shadow_child_id), Some(2));
    assert_eq!(tree.height(child_id), Some(3));
    assert_eq!(
        tree.1
            .get(parent_id)
            .unwrap()
            .child_subtree
            .as_ref()
            .unwrap()
            .shadow_roots,
        &[shadow_parent_id]
    );
    assert_eq!(
        tree.1.get(shadow_child_id).unwrap().slot_for_light_tree,
        Some(parent_id)
    );

    // Remove shadow tree from main tree
    tree.remove_subtree(parent_id);

    // Check that both trees are correct individually
    assert_eq!(tree.height(parent_id), Some(0));
    assert_eq!(tree.height(child_id), Some(1));
    assert_eq!(tree.parent_id(parent_id), None);
    assert_eq!(tree.parent_id(child_id).unwrap(), parent_id);
    assert_eq!(tree.children_ids(parent_id), &[child_id]);

    assert_eq!(tree.height(shadow_parent_id), Some(0));
    assert_eq!(tree.height(shadow_child_id), Some(1));
    assert_eq!(tree.parent_id(shadow_parent_id), None);
    assert_eq!(tree.parent_id(shadow_child_id).unwrap(), shadow_parent_id);
    assert_eq!(tree.children_ids(shadow_parent_id), &[shadow_child_id]);
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
