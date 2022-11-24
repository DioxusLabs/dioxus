use core::panic;
use parking_lot::lock_api::RawMutex as _;
use parking_lot::{RawMutex, RwLock};
use slab::Slab;
use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord)]
pub struct NodeId(pub usize);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Node<T> {
    value: T,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

#[derive(Debug)]
pub struct Tree<T> {
    nodes: Slab<Node<T>>,
    root: NodeId,
}

pub trait TreeView<T>: Sized {
    type Iterator<'a>: Iterator<Item = &'a T>
    where
        T: 'a,
        Self: 'a;
    type IteratorMut<'a>: Iterator<Item = &'a mut T>
    where
        T: 'a,
        Self: 'a;

    fn root(&self) -> NodeId;

    fn get(&self, id: NodeId) -> Option<&T>;

    fn get_unchecked(&self, id: NodeId) -> &T {
        unsafe { self.get(id).unwrap_unchecked() }
    }

    fn get_mut(&mut self, id: NodeId) -> Option<&mut T>;

    fn get_mut_unchecked(&mut self, id: NodeId) -> &mut T {
        unsafe { self.get_mut(id).unwrap_unchecked() }
    }

    fn children<'a>(&'a self, id: NodeId) -> Option<Self::Iterator<'a>>;

    fn children_mut<'a>(&'a mut self, id: NodeId) -> Option<Self::IteratorMut<'a>>;

    fn parent_child_mut<'a>(
        &'a mut self,
        id: NodeId,
    ) -> Option<(&'a mut T, Self::IteratorMut<'a>)> {
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

    fn children_ids<'a>(&'a self, id: NodeId) -> Option<&'a [NodeId]>;

    fn parent(&self, id: NodeId) -> Option<&T>;

    fn parent_mut(&mut self, id: NodeId) -> Option<&mut T>;

    fn node_parent_mut(&mut self, id: NodeId) -> Option<(&mut T, &mut T)> {
        let mut_ptr: *mut Self = self;
        unsafe {
            // Safety: No node has itself as a parent.
            (*mut_ptr)
                .get_mut(id)
                .and_then(|parent| (*mut_ptr).parent_mut(id).map(|children| (parent, children)))
        }
    }

    fn parent_id(&self, id: NodeId) -> Option<NodeId>;

    fn map<T2, F: Fn(&T) -> &T2, FMut: Fn(&mut T) -> &mut T2>(
        &mut self,
        map: F,
        map_mut: FMut,
    ) -> TreeMap<T, T2, Self, F, FMut> {
        TreeMap::new(self, map, map_mut)
    }

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

pub trait TreeLike<T>: TreeView<T> {
    fn new(root: T) -> Self;

    fn add_child(&mut self, parent: NodeId, value: T) -> NodeId;

    fn remove(&mut self, id: NodeId) -> Option<T>;

    fn replace(&mut self, id: NodeId, value: T);

    fn insert_before(&mut self, id: NodeId, value: T) -> NodeId;

    fn insert_after(&mut self, id: NodeId, value: T) -> NodeId;
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

pub struct ChildNodeIteratorMut<'a, T, Tr: TreeView<T> + 'a> {
    tree: *mut Tr,
    children_ids: &'a [NodeId],
    index: usize,
    node_type: PhantomData<T>,
}

unsafe impl<'a, T, Tr: TreeView<T> + 'a> Sync for ChildNodeIteratorMut<'a, T, Tr> {}

impl<'a, T, Tr: TreeView<T>> ChildNodeIteratorMut<'a, T, Tr> {
    fn tree_mut(&mut self) -> &'a mut Tr {
        unsafe { &mut *self.tree }
    }
}

impl<'a, T: 'a, Tr: TreeView<T>> Iterator for ChildNodeIteratorMut<'a, T, Tr> {
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
    type IteratorMut<'a> = ChildNodeIteratorMut<'a, T, Tree<T>> where T: 'a;

    fn root(&self) -> NodeId {
        self.root
    }

    fn get(&self, id: NodeId) -> Option<&T> {
        self.nodes.get(id.0).map(|node| &node.value)
    }

    fn get_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.nodes.get_mut(id.0).map(|node| &mut node.value)
    }

    fn children<'a>(&'a self, id: NodeId) -> Option<Self::Iterator<'a>> {
        self.children_ids(id).map(|children_ids| ChildNodeIterator {
            tree: self,
            children_ids,
            index: 0,
            node_type: PhantomData,
        })
    }

    fn children_mut<'a>(&'a mut self, id: NodeId) -> Option<Self::IteratorMut<'a>> {
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

    fn children_ids<'a>(&'a self, id: NodeId) -> Option<&'a [NodeId]> {
        self.nodes.get(id.0).map(|node| node.children.as_slice())
    }

    fn parent(&self, id: NodeId) -> Option<&T> {
        self.nodes
            .get(id.0)
            .and_then(|node| node.parent.map(|id| self.nodes.get(id.0).unwrap()))
            .map(|node| &node.value)
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

    fn parent_id(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(id.0).and_then(|node| node.parent)
    }

    fn get_unchecked(&self, id: NodeId) -> &T {
        unsafe { &self.nodes.get_unchecked(id.0).value }
    }

    fn get_mut_unchecked(&mut self, id: NodeId) -> &mut T {
        unsafe { &mut self.nodes.get_unchecked_mut(id.0).value }
    }

    fn size(&self) -> usize {
        self.nodes.len()
    }
}

impl<T> TreeLike<T> for Tree<T> {
    fn new(root: T) -> Self {
        let mut nodes = Slab::default();
        let root = NodeId(nodes.insert(Node {
            value: root,
            parent: None,
            children: Vec::new(),
        }));
        Self { nodes, root }
    }

    fn add_child(&mut self, parent: NodeId, value: T) -> NodeId {
        let node = Node {
            value,
            parent: Some(parent),
            children: Vec::new(),
        };
        let id = self.nodes.insert(node);
        self.nodes
            .get_mut(parent.0)
            .unwrap()
            .children
            .push(NodeId(id));
        NodeId(id)
    }

    fn remove(&mut self, id: NodeId) -> Option<T> {
        self.nodes.try_remove(id.0).map(|node| {
            if let Some(parent) = node.parent {
                self.nodes
                    .get_mut(parent.0)
                    .unwrap()
                    .children
                    .retain(|child| child != &id);
            }
            node.value
        })
    }

    fn replace(&mut self, id: NodeId, value: T) {
        let old = self
            .nodes
            .get_mut(id.0)
            .expect("tried to replace a node that doesn't exist");
        old.value = value;
    }

    fn insert_before(&mut self, id: NodeId, value: T) -> NodeId {
        let node = self.nodes.get(id.0).unwrap();
        let parent_id = node.parent.expect("tried to insert before root");
        let new = Node {
            value,
            parent: Some(parent_id),
            children: Vec::new(),
        };
        let new_id = NodeId(self.nodes.insert(new));
        let parent = self.nodes.get_mut(parent_id.0).unwrap();
        let index = parent
            .children
            .iter()
            .position(|child| child == &id)
            .unwrap();
        parent.children.insert(index, new_id);
        new_id
    }

    fn insert_after(&mut self, id: NodeId, value: T) -> NodeId {
        let node = self.nodes.get(id.0).unwrap();
        let parent_id = node.parent.expect("tried to insert before root");
        let new = Node {
            value,
            parent: Some(parent_id),
            children: Vec::new(),
        };
        let new_id = NodeId(self.nodes.insert(new));
        let parent = self.nodes.get_mut(parent_id.0).unwrap();
        let index = parent
            .children
            .iter()
            .position(|child| child == &id)
            .unwrap();
        parent.children.insert(index + 1, new_id);
        new_id
    }
}

pub struct TreeMap<'a, T1, T2, Tr, F, FMut>
where
    Tr: TreeView<T1>,
    F: Fn(&T1) -> &T2,
    FMut: Fn(&mut T1) -> &mut T2,
{
    tree: &'a mut Tr,
    map: F,
    map_mut: FMut,
    in_node_type: PhantomData<T1>,
    out_node_type: PhantomData<T2>,
}

impl<'a, T1, T2, Tr, F, FMut> TreeMap<'a, T1, T2, Tr, F, FMut>
where
    Tr: TreeView<T1>,
    F: Fn(&T1) -> &T2,
    FMut: Fn(&mut T1) -> &mut T2,
{
    pub fn new(tree: &'a mut Tr, map: F, map_mut: FMut) -> Self {
        TreeMap {
            tree,
            map,
            map_mut,
            in_node_type: PhantomData,
            out_node_type: PhantomData,
        }
    }
}

impl<'a, T1, T2, Tr, F, FMut> TreeView<T2> for TreeMap<'a, T1, T2, Tr, F, FMut>
where
    Tr: TreeView<T1>,
    F: Fn(&T1) -> &T2,
    FMut: Fn(&mut T1) -> &mut T2,
{
    type Iterator<'b> = ChildNodeIterator<'b, T2, TreeMap<'a, T1, T2, Tr, F, FMut>>
    where
        T2: 'b,
        Self:'b;
    type IteratorMut<'b> = ChildNodeIteratorMut<'b, T2, TreeMap<'a, T1, T2, Tr, F, FMut>>
    where
        T2: 'b,
        Self:'b;

    fn root(&self) -> NodeId {
        self.tree.root()
    }

    fn get(&self, id: NodeId) -> Option<&T2> {
        self.tree.get(id).map(|node| (self.map)(node))
    }

    fn get_mut(&mut self, id: NodeId) -> Option<&mut T2> {
        self.tree.get_mut(id).map(|node| (self.map_mut)(node))
    }

    fn children<'b>(&'b self, id: NodeId) -> Option<Self::Iterator<'b>> {
        self.children_ids(id).map(|children_ids| ChildNodeIterator {
            tree: self,
            children_ids,
            index: 0,
            node_type: PhantomData,
        })
    }

    fn children_mut<'b>(&'b mut self, id: NodeId) -> Option<Self::IteratorMut<'b>> {
        let raw_ptr = self as *mut Self;
        unsafe {
            // Safety: No node can be a child twice.
            self.children_ids(id)
                .map(|children_ids| ChildNodeIteratorMut {
                    tree: &mut *raw_ptr,
                    children_ids,
                    index: 0,
                    node_type: PhantomData,
                })
        }
    }

    fn children_ids<'b>(&'b self, id: NodeId) -> Option<&'b [NodeId]> {
        self.tree.children_ids(id)
    }

    fn parent(&self, id: NodeId) -> Option<&T2> {
        self.tree.parent(id).map(|node| (self.map)(node))
    }

    fn parent_mut(&mut self, id: NodeId) -> Option<&mut T2> {
        self.tree.parent_mut(id).map(|node| (self.map_mut)(node))
    }

    fn parent_id(&self, id: NodeId) -> Option<NodeId> {
        self.tree.parent_id(id)
    }

    fn get_unchecked(&self, id: NodeId) -> &T2 {
        (self.map)(self.tree.get_unchecked(id))
    }

    fn get_mut_unchecked(&mut self, id: NodeId) -> &mut T2 {
        (self.map_mut)(self.tree.get_mut_unchecked(id))
    }

    fn size(&self) -> usize {
        self.tree.size()
    }
}

/// A view into a tree that can be shared between multiple threads. Nodes are locked invividually.
pub struct SharedView<'a, T, Tr: TreeView<T>> {
    tree: Arc<UnsafeCell<&'a mut Tr>>,
    node_locks: Arc<RwLock<Vec<RawMutex>>>,
    node_type: PhantomData<T>,
}

impl<'a, T, Tr: TreeView<T>> SharedView<'a, T, Tr> {
    /// Checks if a node is currently locked. Returns None if the node does not exist.
    pub fn check_lock(&self, id: NodeId) -> Option<bool> {
        let mut locks = self.node_locks.read();
        locks.get(id.0).map(|lock| lock.is_locked())
    }
}

unsafe impl<'a, T, Tr: TreeView<T>> Send for SharedView<'a, T, Tr> {}
unsafe impl<'a, T, Tr: TreeView<T>> Sync for SharedView<'a, T, Tr> {}
impl<'a, T, Tr: TreeView<T>> Clone for SharedView<'a, T, Tr> {
    fn clone(&self) -> Self {
        Self {
            tree: self.tree.clone(),
            node_locks: self.node_locks.clone(),
            node_type: PhantomData,
        }
    }
}

impl<'a, T, Tr: TreeView<T>> SharedView<'a, T, Tr> {
    pub fn new(tree: &'a mut Tr) -> Self {
        let tree = Arc::new(UnsafeCell::new(tree));
        let mut node_locks = Vec::new();
        for _ in 0..unsafe { (*tree.get()).size() } {
            node_locks.push(RawMutex::INIT);
        }
        Self {
            tree,
            node_locks: Arc::new(RwLock::new(node_locks)),
            node_type: PhantomData,
        }
    }

    fn lock_node(&self, node: NodeId) {
        let read = self.node_locks.read();
        let lock = read.get(node.0);
        match lock {
            Some(lock) => lock.lock(),
            None => {
                drop(read);
                let mut write = self.node_locks.write();
                write.resize_with(node.0 + 1, || RawMutex::INIT);
                unsafe { write.get_unchecked(node.0).lock() }
            }
        }
    }

    fn unlock_node(&self, node: NodeId) {
        let read = self.node_locks.read();
        let lock = read.get(node.0);
        match lock {
            Some(lock) => unsafe { lock.unlock() },
            None => {
                panic!("unlocking node that was not locked")
            }
        }
    }

    fn with_node<R>(&self, node_id: NodeId, f: impl FnOnce(&'a mut Tr) -> R) -> R {
        self.lock_node(node_id);
        let tree = unsafe { &mut *self.tree.get() };
        let r = f(tree);
        self.unlock_node(node_id);
        r
    }
}

impl<'a, T, Tr: TreeView<T>> TreeView<T> for SharedView<'a, T, Tr> {
    type Iterator<'b> = Tr::Iterator<'b> where T: 'b, Self: 'b;

    type IteratorMut<'b>=Tr::IteratorMut<'b>
    where
        T: 'b,
        Self: 'b;

    fn root(&self) -> NodeId {
        unsafe { (*self.tree.get()).root() }
    }

    fn get(&self, id: NodeId) -> Option<&T> {
        self.with_node(id, |t| t.get(id))
    }

    fn get_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.with_node(id, |t| t.get_mut(id))
    }

    fn children<'b>(&'b self, id: NodeId) -> Option<Self::Iterator<'b>> {
        self.with_node(id, |t| t.children(id))
    }

    fn children_mut<'b>(&'b mut self, id: NodeId) -> Option<Self::IteratorMut<'b>> {
        self.with_node(id, |t| t.children_mut(id))
    }

    fn children_ids<'b>(&'b self, id: NodeId) -> Option<&'b [NodeId]> {
        self.with_node(id, |t| t.children_ids(id))
    }

    fn parent(&self, id: NodeId) -> Option<&T> {
        self.with_node(id, |t| t.get(id))
    }

    fn parent_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.with_node(id, |t| t.parent_mut(id))
    }

    fn parent_id(&self, id: NodeId) -> Option<NodeId> {
        self.with_node(id, |t| t.parent_id(id))
    }

    fn size(&self) -> usize {
        unsafe { (*self.tree.get()).size() }
    }
}

#[test]
fn creation() {
    let mut tree = Tree::new(1);
    let parent = tree.root();
    let child = tree.add_child(parent, 0);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 2);
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
    let child = tree.add_child(parent, 2);
    let before = tree.insert_before(child, 1);
    let after = tree.insert_after(child, 3);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 4);
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
    let child = tree.add_child(parent, 2);
    let before = tree.insert_before(child, 1);
    let after = tree.insert_after(child, 3);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 4);
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
    assert_eq!(*tree.get(parent).unwrap(), 0);
    assert_eq!(tree.get(before), None);
    assert_eq!(*tree.get(after).unwrap(), 3);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[after]);

    tree.remove(after);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 1);
    assert_eq!(*tree.get(parent).unwrap(), 0);
    assert_eq!(tree.get(after), None);
    assert_eq!(tree.children_ids(parent).unwrap(), &[]);
}

#[test]
fn shared_view() {
    use std::thread;
    let mut tree = Tree::new(1);
    let parent = tree.root();
    let child = tree.add_child(parent, 0);

    let shared = SharedView::new(&mut tree);

    thread::scope(|s| {
        let (mut shared1, mut shared2, mut shared3) =
            (shared.clone(), shared.clone(), shared.clone());
        s.spawn(move || {
            assert_eq!(*shared1.get_mut(parent).unwrap(), 1);
            assert_eq!(*shared1.get_mut(child).unwrap(), 0);
        });
        s.spawn(move || {
            assert_eq!(*shared2.get_mut(child).unwrap(), 0);
            assert_eq!(*shared2.get_mut(parent).unwrap(), 1);
        });
        s.spawn(move || {
            assert_eq!(*shared3.get_mut(parent).unwrap(), 1);
            assert_eq!(*shared3.get_mut(child).unwrap(), 0);
        });
    });
}

#[test]
fn map() {
    #[derive(Debug, PartialEq)]
    struct Value {
        value: i32,
    }
    impl Value {
        fn new(value: i32) -> Self {
            Self { value }
        }
    }
    let mut tree = Tree::new(Value::new(1));
    let parent = tree.root();
    let child = tree.add_child(parent, Value::new(0));

    let mut mapped = tree.map(|x| &x.value, |x| &mut x.value);

    *mapped.get_mut(child).unwrap() = 1;
    *mapped.get_mut(parent).unwrap() = 2;

    assert_eq!(*tree.get(parent).unwrap(), Value::new(2));
    assert_eq!(*tree.get(child).unwrap(), Value::new(1));
}

#[test]
fn traverse_depth_first() {
    let mut tree = Tree::new(0);
    let parent = tree.root();
    let child1 = tree.add_child(parent, 1);
    tree.add_child(child1, 2);
    let child2 = tree.add_child(parent, 3);
    tree.add_child(child2, 4);

    let mut node_count = 0;
    tree.traverse_depth_first(move |node| {
        assert_eq!(*node, node_count);
        node_count += 1;
    });
}

#[test]
fn traverse_breadth_first() {
    let mut tree = Tree::new(0);
    let parent = tree.root();
    let child1 = tree.add_child(parent, 1);
    tree.add_child(child1, 3);
    let child2 = tree.add_child(parent, 2);
    tree.add_child(child2, 4);

    let mut node_count = 0;
    tree.traverse_breadth_first(move |node| {
        assert_eq!(*node, node_count);
        node_count += 1;
    });
}
