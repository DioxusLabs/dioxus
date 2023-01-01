use slab::Slab;
use std::collections::VecDeque;
use std::marker::PhantomData;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord)]
pub struct NodeId(pub usize);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Node<T> {
    value: T,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    height: u16,
}

#[derive(Debug)]
pub struct Tree<T> {
    nodes: Slab<Node<T>>,
    root: NodeId,
}

impl<T> Tree<T> {
    pub fn new(root: T) -> Self {
        let mut nodes = Slab::default();
        let root = NodeId(nodes.insert(Node {
            value: root,
            parent: None,
            children: Vec::new(),
            height: 0,
        }));
        Self { nodes, root }
    }

    fn try_remove(&mut self, id: NodeId) -> Option<Node<T>> {
        self.nodes.try_remove(id.0).map(|node| {
            if let Some(parent) = node.parent {
                self.nodes
                    .get_mut(parent.0)
                    .unwrap()
                    .children
                    .retain(|child| child != &id);
            }
            for child in &node.children {
                self.remove_recursive(*child);
            }
            node
        })
    }

    fn remove_recursive(&mut self, node: NodeId) {
        let node = self.nodes.remove(node.0);
        for child in node.children {
            self.remove_recursive(child);
        }
    }

    fn set_height(&mut self, node: NodeId, height: u16) {
        let self_mut = self as *mut Self;
        let node = self.nodes.get_mut(node.0).unwrap();
        node.height = height;
        unsafe {
            // Safety: No node has itself as a child
            for child in &node.children {
                (*self_mut).set_height(*child, height + 1);
            }
        }
    }
}

pub trait TreeView<T>: Sized {
    type Iterator<'a>: Iterator<Item = &'a T>
    where
        T: 'a,
        Self: 'a;

    fn root(&self) -> NodeId;

    fn contains(&self, id: NodeId) -> bool {
        self.get(id).is_some()
    }

    fn get(&self, id: NodeId) -> Option<&T>;

    fn get_unchecked(&self, id: NodeId) -> &T {
        unsafe { self.get(id).unwrap_unchecked() }
    }

    fn children(&self, id: NodeId) -> Option<Self::Iterator<'_>>;

    fn children_ids(&self, id: NodeId) -> Option<&[NodeId]>;

    fn parent(&self, id: NodeId) -> Option<&T>;

    fn parent_id(&self, id: NodeId) -> Option<NodeId>;

    fn height(&self, id: NodeId) -> Option<u16>;

    fn size(&self) -> usize;

    fn traverse_depth_first(&self, mut f: impl FnMut(&T)) {
        let mut stack = vec![self.root()];
        while let Some(id) = stack.pop() {
            if let Some(node) = self.get(id) {
                f(node);
                if let Some(children) = self.children_ids(id) {
                    stack.extend(children.iter().copied().rev());
                }
            }
        }
    }

    fn traverse_breadth_first(&self, mut f: impl FnMut(&T)) {
        let mut queue = VecDeque::new();
        queue.push_back(self.root());
        while let Some(id) = queue.pop_front() {
            if let Some(node) = self.get(id) {
                f(node);
                if let Some(children) = self.children_ids(id) {
                    for id in children {
                        queue.push_back(*id);
                    }
                }
            }
        }
    }
}

pub trait TreeViewMut<T>: Sized + TreeView<T> {
    type IteratorMut<'a>: Iterator<Item = &'a mut T>
    where
        T: 'a,
        Self: 'a;

    fn get_mut(&mut self, id: NodeId) -> Option<&mut T>;

    fn get_mut_unchecked(&mut self, id: NodeId) -> &mut T {
        unsafe { self.get_mut(id).unwrap_unchecked() }
    }

    fn children_mut(&mut self, id: NodeId) -> Option<<Self as TreeViewMut<T>>::IteratorMut<'_>>;

    fn parent_child_mut(
        &mut self,
        id: NodeId,
    ) -> Option<(&mut T, <Self as TreeViewMut<T>>::IteratorMut<'_>)> {
        let mut_ptr: *mut Self = self;
        unsafe {
            // Safety: No node has itself as a child.
            (*mut_ptr).get_mut(id).and_then(|parent| {
                (*mut_ptr)
                    .children_mut(id)
                    .map(|children| (parent, children))
            })
        }
    }

    fn parent_mut(&mut self, id: NodeId) -> Option<&mut T>;

    fn node_parent_mut(&mut self, id: NodeId) -> Option<(&mut T, Option<&mut T>)>;

    fn node_parent_children_mut(
        &mut self,
        id: NodeId,
    ) -> Option<(
        &mut T,
        Option<&mut T>,
        Option<<Self as TreeViewMut<T>>::IteratorMut<'_>>,
    )>;

    fn traverse_depth_first_mut(&mut self, mut f: impl FnMut(&mut T)) {
        let mut stack = vec![self.root()];
        while let Some(id) = stack.pop() {
            if let Some(node) = self.get_mut(id) {
                f(node);
                if let Some(children) = self.children_ids(id) {
                    stack.extend(children.iter().copied().rev());
                }
            }
        }
    }

    fn traverse_breadth_first_mut(&mut self, mut f: impl FnMut(&mut T)) {
        let mut queue = VecDeque::new();
        queue.push_back(self.root());
        while let Some(id) = queue.pop_front() {
            if let Some(node) = self.get_mut(id) {
                f(node);
                if let Some(children) = self.children_ids(id) {
                    for id in children {
                        queue.push_back(*id);
                    }
                }
            }
        }
    }
}

pub trait TreeLike<T> {
    fn create_node(&mut self, value: T) -> NodeId;

    fn add_child(&mut self, parent: NodeId, child: NodeId);

    fn remove(&mut self, id: NodeId) -> Option<T>;

    fn remove_all_children(&mut self, id: NodeId) -> Vec<T>;

    fn replace(&mut self, old: NodeId, new: NodeId);

    fn insert_before(&mut self, id: NodeId, new: NodeId);

    fn insert_after(&mut self, id: NodeId, new: NodeId);
}

pub struct ChildNodeIterator<'a, T, Tr: TreeView<T>> {
    tree: &'a Tr,
    children_ids: &'a [NodeId],
    index: usize,
    node_type: PhantomData<T>,
}

impl<'a, T: 'a, Tr: TreeView<T>> Iterator for ChildNodeIterator<'a, T, Tr> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.children_ids.get(self.index).map(|id| {
            self.index += 1;
            self.tree.get_unchecked(*id)
        })
    }
}

pub struct ChildNodeIteratorMut<'a, T, Tr: TreeViewMut<T> + 'a> {
    tree: *mut Tr,
    children_ids: &'a [NodeId],
    index: usize,
    node_type: PhantomData<T>,
}

unsafe impl<'a, T, Tr: TreeViewMut<T> + 'a> Sync for ChildNodeIteratorMut<'a, T, Tr> {}

impl<'a, T, Tr: TreeViewMut<T>> ChildNodeIteratorMut<'a, T, Tr> {
    fn tree_mut(&mut self) -> &'a mut Tr {
        unsafe { &mut *self.tree }
    }
}

impl<'a, T: 'a, Tr: TreeViewMut<T>> Iterator for ChildNodeIteratorMut<'a, T, Tr> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        let owned = self.children_ids.get(self.index).copied();
        match owned {
            Some(id) => {
                self.index += 1;

                Some(self.tree_mut().get_mut_unchecked(id))
            }
            None => None,
        }
    }
}

impl<T> TreeView<T> for Tree<T> {
    type Iterator<'a> = ChildNodeIterator<'a, T, Tree<T>> where T: 'a;

    fn root(&self) -> NodeId {
        self.root
    }

    fn get(&self, id: NodeId) -> Option<&T> {
        self.nodes.get(id.0).map(|node| &node.value)
    }

    fn children(&self, id: NodeId) -> Option<Self::Iterator<'_>> {
        self.children_ids(id).map(|children_ids| ChildNodeIterator {
            tree: self,
            children_ids,
            index: 0,
            node_type: PhantomData,
        })
    }

    fn children_ids(&self, id: NodeId) -> Option<&[NodeId]> {
        self.nodes.get(id.0).map(|node| node.children.as_slice())
    }

    fn parent(&self, id: NodeId) -> Option<&T> {
        self.nodes
            .get(id.0)
            .and_then(|node| node.parent.map(|id| self.nodes.get(id.0).unwrap()))
            .map(|node| &node.value)
    }

    fn parent_id(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(id.0).and_then(|node| node.parent)
    }

    fn height(&self, id: NodeId) -> Option<u16> {
        self.nodes.get(id.0).map(|n| n.height)
    }

    fn get_unchecked(&self, id: NodeId) -> &T {
        unsafe { &self.nodes.get_unchecked(id.0).value }
    }

    fn size(&self) -> usize {
        self.nodes.len()
    }
}

impl<T> TreeViewMut<T> for Tree<T> {
    type IteratorMut<'a> = ChildNodeIteratorMut<'a, T, Tree<T>> where T: 'a;

    fn get_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.nodes.get_mut(id.0).map(|node| &mut node.value)
    }

    fn parent_mut(&mut self, id: NodeId) -> Option<&mut T> {
        let self_ptr = self as *mut Self;
        unsafe {
            // Safety: No node has itself as a parent.
            self.nodes
                .get_mut(id.0)
                .and_then(move |node| {
                    node.parent
                        .map(move |id| (*self_ptr).nodes.get_mut(id.0).unwrap())
                })
                .map(|node| &mut node.value)
        }
    }

    fn children_mut(&mut self, id: NodeId) -> Option<Self::IteratorMut<'_>> {
        let raw_ptr = self as *mut Self;
        unsafe {
            // Safety: No node will appear as a child twice
            self.children_ids(id)
                .map(|children_ids| ChildNodeIteratorMut {
                    tree: &mut *raw_ptr,
                    children_ids,
                    index: 0,
                    node_type: PhantomData,
                })
        }
    }

    fn get_mut_unchecked(&mut self, id: NodeId) -> &mut T {
        unsafe { &mut self.nodes.get_unchecked_mut(id.0).value }
    }

    fn node_parent_mut(&mut self, id: NodeId) -> Option<(&mut T, Option<&mut T>)> {
        if let Some(parent_id) = self.parent_id(id) {
            self.nodes
                .get2_mut(id.0, parent_id.0)
                .map(|(node, parent)| (&mut node.value, Some(&mut parent.value)))
        } else {
            self.nodes.get_mut(id.0).map(|node| (&mut node.value, None))
        }
    }

    fn node_parent_children_mut(
        &mut self,
        id: NodeId,
    ) -> Option<(
        &mut T,
        Option<&mut T>,
        Option<<Self as TreeViewMut<T>>::IteratorMut<'_>>,
    )> {
        // Safety: No node has itself as a parent.
        let unbounded_self = unsafe { &mut *(self as *mut Self) };
        self.node_parent_mut(id).map(move |(node, parent)| {
            let children = unbounded_self.children_mut(id);
            (node, parent, children)
        })
    }
}

impl<T> TreeLike<T> for Tree<T> {
    fn create_node(&mut self, value: T) -> NodeId {
        NodeId(self.nodes.insert(Node {
            value,
            parent: None,
            children: Vec::new(),
            height: 0,
        }))
    }

    fn add_child(&mut self, parent: NodeId, new: NodeId) {
        self.nodes.get_mut(new.0).unwrap().parent = Some(parent);
        let parent = self.nodes.get_mut(parent.0).unwrap();
        parent.children.push(new);
        let height = parent.height + 1;
        self.set_height(new, height);
    }

    fn remove(&mut self, id: NodeId) -> Option<T> {
        self.try_remove(id).map(|node| node.value)
    }

    fn remove_all_children(&mut self, id: NodeId) -> Vec<T> {
        let mut children = Vec::new();
        let self_mut = self as *mut Self;
        for child in self.children_ids(id).unwrap() {
            unsafe {
                // Safety: No node has itself as a child
                children.push((*self_mut).remove(*child).unwrap());
            }
        }
        children
    }

    fn replace(&mut self, old_id: NodeId, new_id: NodeId) {
        // remove the old node
        let old = self
            .try_remove(old_id)
            .expect("tried to replace a node that doesn't exist");
        // update the parent's link to the child
        if let Some(parent_id) = old.parent {
            let parent = self.nodes.get_mut(parent_id.0).unwrap();
            for id in &mut parent.children {
                if *id == old_id {
                    *id = new_id;
                }
            }
            let height = parent.height + 1;
            self.set_height(new_id, height);
        }
    }

    fn insert_before(&mut self, id: NodeId, new: NodeId) {
        let node = self.nodes.get(id.0).unwrap();
        let parent_id = node.parent.expect("tried to insert before root");
        self.nodes.get_mut(new.0).unwrap().parent = Some(parent_id);
        let parent = self.nodes.get_mut(parent_id.0).unwrap();
        let index = parent
            .children
            .iter()
            .position(|child| child == &id)
            .unwrap();
        parent.children.insert(index, new);
        let height = parent.height + 1;
        self.set_height(new, height);
    }

    fn insert_after(&mut self, id: NodeId, new: NodeId) {
        let node = self.nodes.get(id.0).unwrap();
        let parent_id = node.parent.expect("tried to insert before root");
        self.nodes.get_mut(new.0).unwrap().parent = Some(parent_id);
        let parent = self.nodes.get_mut(parent_id.0).unwrap();
        let index = parent
            .children
            .iter()
            .position(|child| child == &id)
            .unwrap();
        parent.children.insert(index + 1, new);
        let height = parent.height + 1;
        self.set_height(new, height);
    }
}
#[test]
fn creation() {
    let mut tree = Tree::new(1);
    let parent = tree.root();
    let child = tree.create_node(0);
    tree.add_child(parent, child);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 2);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(child), Some(1));
    assert_eq!(*tree.get(parent).unwrap(), 1);
    assert_eq!(*tree.get(child).unwrap(), 0);
    assert_eq!(tree.parent_id(parent), None);
    assert_eq!(tree.parent_id(child).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[child]);
}

#[test]
fn insertion() {
    let mut tree = Tree::new(0);
    let parent = tree.root();
    let child = tree.create_node(2);
    tree.add_child(parent, child);
    let before = tree.create_node(1);
    tree.insert_before(child, before);
    let after = tree.create_node(3);
    tree.insert_after(child, after);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 4);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(child), Some(1));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(*tree.get(parent).unwrap(), 0);
    assert_eq!(*tree.get(before).unwrap(), 1);
    assert_eq!(*tree.get(child).unwrap(), 2);
    assert_eq!(*tree.get(after).unwrap(), 3);
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(child).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[before, child, after]);
}

#[test]
fn deletion() {
    let mut tree = Tree::new(0);
    let parent = tree.root();
    let child = tree.create_node(2);
    tree.add_child(parent, child);
    let before = tree.create_node(1);
    tree.insert_before(child, before);
    let after = tree.create_node(3);
    tree.insert_after(child, after);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 4);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(child), Some(1));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(*tree.get(parent).unwrap(), 0);
    assert_eq!(*tree.get(before).unwrap(), 1);
    assert_eq!(*tree.get(child).unwrap(), 2);
    assert_eq!(*tree.get(after).unwrap(), 3);
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(child).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[before, child, after]);

    tree.remove(child);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 3);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(*tree.get(parent).unwrap(), 0);
    assert_eq!(*tree.get(before).unwrap(), 1);
    assert_eq!(tree.get(child), None);
    assert_eq!(*tree.get(after).unwrap(), 3);
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[before, after]);

    tree.remove(before);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 2);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(*tree.get(parent).unwrap(), 0);
    assert_eq!(tree.get(before), None);
    assert_eq!(*tree.get(after).unwrap(), 3);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[after]);

    tree.remove(after);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 1);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(*tree.get(parent).unwrap(), 0);
    assert_eq!(tree.get(after), None);
    assert_eq!(tree.children_ids(parent).unwrap(), &[]);
}

#[test]
fn traverse_depth_first() {
    let mut tree = Tree::new(0);
    let parent = tree.root();
    let child1 = tree.create_node(1);
    tree.add_child(parent, child1);
    let grandchild1 = tree.create_node(2);
    tree.add_child(child1, grandchild1);
    let child2 = tree.create_node(3);
    tree.add_child(parent, child2);
    let grandchild2 = tree.create_node(4);
    tree.add_child(child2, grandchild2);

    let mut node_count = 0;
    tree.traverse_depth_first(move |node| {
        assert_eq!(*node, node_count);
        node_count += 1;
    });
}
