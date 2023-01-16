use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use rustc_hash::FxHasher;
use std::any::{Any, TypeId};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::hash::BuildHasherDefault;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Node {
    parent: Option<usize>,
    children: Vec<usize>,
    height: u16,
}

pub struct NodeView<'a> {
    tree: &'a Tree,
    id: usize,
}

impl NodeView<'_> {
    pub fn insert<T>(&self, data: T)
    where
        T: Any,
    {
        self.tree.nodes.get_slab_mut::<T>().insert(self.id, data);
    }

    pub fn get<T>(&self) -> MappedRwLockReadGuard<'_, T>
    where
        T: Any,
    {
        MappedRwLockReadGuard::map(self.tree.nodes.get_slab::<T>(), |slab| {
            slab.get(self.id).unwrap()
        })
    }

    pub fn get_mut<T>(&self) -> MappedRwLockWriteGuard<'_, T>
    where
        T: Any,
    {
        MappedRwLockWriteGuard::map(self.tree.nodes.get_slab_mut::<T>(), |slab| {
            slab.get_mut(self.id).unwrap()
        })
    }

    pub fn height(&self) -> u16 {
        self.get::<Node>().height
    }

    pub fn children_ids(&self) -> Vec<usize> {
        self.get::<Node>().children.clone()
    }

    pub fn parent_id(&self) -> Option<usize> {
        self.get::<Node>().parent
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

#[derive(Debug)]
pub struct Tree {
    nodes: AnySlab,
    root: usize,
}

impl Tree {
    pub fn new() -> Self {
        let mut nodes = AnySlab::default();
        let mut node = nodes.insert();
        node.insert(Node {
            parent: None,
            children: Vec::new(),
            height: 0,
        });
        let root = node.id();
        Self { nodes, root }
    }

    fn node_slab(&self) -> MappedRwLockReadGuard<'_, SlabStorage<Node>> {
        self.nodes.get_slab()
    }

    pub fn get_node_data(&self, id: usize) -> MappedRwLockReadGuard<'_, Node> {
        MappedRwLockReadGuard::map(self.node_slab(), |slab| slab.get(id).unwrap())
    }

    fn node_slab_mut(&self) -> MappedRwLockWriteGuard<'_, SlabStorage<Node>> {
        self.nodes.get_slab_mut()
    }

    pub fn get_node_data_mut(&self, id: usize) -> MappedRwLockWriteGuard<'_, Node> {
        MappedRwLockWriteGuard::map(self.node_slab_mut(), |slab| slab.get_mut(id).unwrap())
    }

    pub fn remove(&mut self, id: usize) {
        fn recurse(tree: &mut Tree, id: usize) {
            let children = { tree.get_node_data(id).children.clone() };
            for child in children {
                recurse(tree, child);
            }

            tree.nodes.remove(id);
        }
        {
            let mut node_data_mut = self.node_slab_mut();
            if let Some(parent) = node_data_mut.get(id).unwrap().parent {
                let parent = node_data_mut.get_mut(parent).unwrap();
                parent.children.retain(|&child| child != id);
            }
        }

        recurse(self, id);
    }

    fn set_height(&self, node: usize, height: u16) {
        let children = {
            let mut node = self.get_node_data_mut(node);
            node.height = height;
            node.children.clone()
        };
        for child in children {
            self.set_height(child, height + 1);
        }
    }

    pub fn create_node(&mut self) -> Entry<'_> {
        let mut node = self.nodes.insert();
        node.insert(Node {
            parent: None,
            children: Vec::new(),
            height: 0,
        });
        node
    }

    pub fn add_child(&mut self, parent: usize, new: usize) {
        let mut node_state = self.node_slab_mut();
        node_state.get_mut(new).unwrap().parent = Some(parent);
        let parent = node_state.get_mut(parent).unwrap();
        parent.children.push(new);
        let height = parent.height + 1;
        drop(node_state);
        self.set_height(new, height);
    }

    pub fn replace(&mut self, old_id: usize, new_id: usize) {
        {
            let mut node_state = self.node_slab_mut();
            // update the parent's link to the child
            if let Some(parent_id) = node_state.get(old_id).unwrap().parent {
                let parent = node_state.get_mut(parent_id).unwrap();
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

    pub fn insert_before(&mut self, old_id: usize, new_id: usize) {
        let mut node_state = self.node_slab_mut();
        let old_node = node_state.get(old_id).unwrap();
        let parent_id = old_node.parent.expect("tried to insert before root");
        node_state.get_mut(new_id).unwrap().parent = Some(parent_id);
        let parent = node_state.get_mut(parent_id).unwrap();
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

    pub fn insert_after(&mut self, old_id: usize, new_id: usize) {
        let mut node_state = self.node_slab_mut();
        let old_node = node_state.get(old_id).unwrap();
        let parent_id = old_node.parent.expect("tried to insert before root");
        node_state.get_mut(new_id).unwrap().parent = Some(parent_id);
        let parent = node_state.get_mut(parent_id).unwrap();
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

    pub fn insert<T: Any>(&mut self, id: usize, value: T) {
        self.nodes.add(id, value);
    }

    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    pub fn state_view<T: Any>(&self) -> TreeStateView<'_, T> {
        TreeStateView {
            nodes_data: self.node_slab(),
            nodes: self.nodes.get_slab(),
            root: self.root,
        }
    }

    pub fn state_view_mut<T: Any>(&mut self) -> TreeStateViewMut<'_, T> {
        TreeStateViewMut {
            nodes_data: self.node_slab(),
            nodes: self.nodes.get_slab_mut(),
            root: self.root,
        }
    }
}

impl NodeDataView for Tree {
    fn root(&self) -> usize {
        self.root
    }

    fn parent_id(&self, id: usize) -> Option<usize> {
        self.get_node_data(id).parent
    }

    fn children_ids(&self, id: usize) -> Option<Vec<usize>> {
        Some(self.get_node_data(id).children.clone())
    }

    fn height(&self, id: usize) -> Option<u16> {
        Some(self.get_node_data(id).height)
    }
}

pub trait NodeDataView {
    fn root(&self) -> usize;

    fn parent_id(&self, id: usize) -> Option<usize>;

    fn children_ids(&self, id: usize) -> Option<Vec<usize>>;

    fn height(&self, id: usize) -> Option<u16>;
}

pub trait TreeView<T>: Sized + NodeDataView {
    fn get(&self, id: usize) -> Option<&T>;

    fn get_unchecked(&self, id: usize) -> &T {
        unsafe { self.get(id).unwrap_unchecked() }
    }

    fn children(&self, id: usize) -> Option<Vec<&T>>;

    fn parent(&self, id: usize) -> Option<&T>;

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
                        queue.push_back(id);
                    }
                }
            }
        }
    }
}

pub trait TreeViewMut<T>: Sized + TreeView<T> {
    fn get_mut(&mut self, id: usize) -> Option<&mut T>;

    unsafe fn get_mut_unchecked(&mut self, id: usize) -> &mut T {
        unsafe { self.get_mut(id).unwrap_unchecked() }
    }

    fn children_mut(&mut self, id: usize) -> Option<Vec<&mut T>>;

    fn parent_child_mut(&mut self, id: usize) -> Option<(&mut T, Vec<&mut T>)> {
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

    fn parent_mut(&mut self, id: usize) -> Option<&mut T>;

    fn node_parent_mut(&mut self, id: usize) -> Option<(&mut T, Option<&mut T>)>;

    #[allow(clippy::type_complexity)]
    fn node_parent_children_mut(
        &mut self,
        id: usize,
    ) -> Option<(&mut T, Option<&mut T>, Option<Vec<&mut T>>)>;

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
                        queue.push_back(id);
                    }
                }
            }
        }
    }
}

pub trait TreeLike {
    fn create_node(&mut self) -> Entry;

    fn add_child(&mut self, parent: usize, child: usize);

    fn remove(&mut self, id: usize);

    fn replace(&mut self, old: usize, new: usize);

    fn insert_before(&mut self, id: usize, new: usize);

    fn insert_after(&mut self, id: usize, new: usize);
}

pub struct TreeStateView<'a, T> {
    nodes_data: MappedRwLockReadGuard<'a, SlabStorage<Node>>,
    nodes: MappedRwLockReadGuard<'a, SlabStorage<T>>,
    root: usize,
}

pub struct TreeStateViewMut<'a, T> {
    nodes_data: MappedRwLockReadGuard<'a, SlabStorage<Node>>,
    nodes: MappedRwLockWriteGuard<'a, SlabStorage<T>>,
    root: usize,
}

impl<'a, T> NodeDataView for TreeStateView<'a, T> {
    fn root(&self) -> usize {
        self.root
    }

    fn children_ids(&self, id: usize) -> Option<Vec<usize>> {
        self.nodes_data.get(id).map(|node| node.children.clone())
    }

    fn parent_id(&self, id: usize) -> Option<usize> {
        self.nodes_data.get(id).and_then(|node| node.parent)
    }

    fn height(&self, id: usize) -> Option<u16> {
        self.nodes_data.get(id).map(|n| n.height)
    }
}

impl<'a, T> TreeView<T> for TreeStateView<'a, T> {
    fn get(&self, id: usize) -> Option<&T> {
        self.nodes.get(id)
    }

    fn children(&self, id: usize) -> Option<Vec<&T>> {
        let ids = self.children_ids(id);
        ids.map(|ids| ids.iter().map(|id| self.get_unchecked(*id)).collect())
    }

    fn parent(&self, id: usize) -> Option<&T> {
        self.nodes_data
            .get(id)
            .and_then(|node| node.parent.map(|id| self.nodes.get(id).unwrap()))
    }

    fn get_unchecked(&self, id: usize) -> &T {
        unsafe { &self.nodes.get_unchecked(id) }
    }
}

impl<'a, T> NodeDataView for TreeStateViewMut<'a, T> {
    fn root(&self) -> usize {
        self.root
    }

    fn children_ids(&self, id: usize) -> Option<Vec<usize>> {
        self.nodes_data.get(id).map(|node| node.children.clone())
    }

    fn parent_id(&self, id: usize) -> Option<usize> {
        self.nodes_data.get(id).and_then(|node| node.parent)
    }

    fn height(&self, id: usize) -> Option<u16> {
        self.nodes_data.get(id).map(|n| n.height)
    }
}

impl<'a, T> TreeView<T> for TreeStateViewMut<'a, T> {
    fn get(&self, id: usize) -> Option<&T> {
        self.nodes.get(id)
    }

    fn children(&self, id: usize) -> Option<Vec<&T>> {
        let ids = self.children_ids(id);
        ids.map(|ids| ids.iter().map(|id| self.get_unchecked(*id)).collect())
    }
    fn parent(&self, id: usize) -> Option<&T> {
        self.nodes_data
            .get(id)
            .and_then(|node| node.parent.map(|id| self.nodes.get(id).unwrap()))
    }

    fn get_unchecked(&self, id: usize) -> &T {
        unsafe { &self.nodes.get_unchecked(id) }
    }
}

impl<'a, T> TreeViewMut<T> for TreeStateViewMut<'a, T> {
    fn get_mut(&mut self, id: usize) -> Option<&mut T> {
        self.nodes.get_mut(id)
    }

    fn parent_mut(&mut self, id: usize) -> Option<&mut T> {
        let parent_id = self.parent_id(id)?;
        unsafe { Some(self.get_mut_unchecked(parent_id)) }
    }

    fn children_mut(&mut self, id: usize) -> Option<Vec<&mut T>> {
        // Safety: No node has itself as a parent.
        if let Some(children_ids) = self.children_ids(id) {
            let children_ids = children_ids.to_vec();
            unsafe {
                self.nodes
                    .get_many_mut_unchecked(children_ids.into_iter().rev().map(|id| id))
            }
        } else {
            None
        }
    }

    unsafe fn get_mut_unchecked(&mut self, id: usize) -> &mut T {
        unsafe { self.nodes.get_unchecked_mut(id) }
    }

    fn node_parent_mut(&mut self, id: usize) -> Option<(&mut T, Option<&mut T>)> {
        if let Some(parent_id) = self.parent_id(id) {
            self.nodes
                .get2_mut(id, parent_id)
                .map(|(node, parent)| (node, Some(parent)))
        } else {
            self.nodes.get_mut(id).map(|node| (node, None))
        }
    }

    fn node_parent_children_mut(
        &mut self,
        id: usize,
    ) -> Option<(&mut T, Option<&mut T>, Option<Vec<&mut T>>)> {
        // Safety: No node has itself as a parent.
        let unbounded_self = unsafe { &mut *(self as *mut Self) };
        self.node_parent_mut(id).map(move |(node, parent)| {
            let children = unbounded_self.children_mut(id);
            (node, parent, children)
        })
    }

    fn parent_child_mut(&mut self, id: usize) -> Option<(&mut T, Vec<&mut T>)> {
        // Safety: No node will appear as a child twice
        if let Some(children_ids) = self.children_ids(id) {
            debug_assert!(!children_ids.iter().any(|child_id| *child_id == id));
            let mut borrowed = unsafe {
                let as_vec = children_ids.to_vec();
                self.nodes
                    .get_many_mut_unchecked(
                        as_vec.into_iter().map(|id| id).chain(std::iter::once(id)),
                    )
                    .unwrap()
            };
            let node = borrowed.pop().unwrap();
            Some((node, borrowed))
        } else {
            None
        }
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

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 2);
    assert_eq!(tree.height(parent_id), Some(0));
    assert_eq!(tree.height(child_id), Some(1));
    assert_eq!(tree.parent_id(parent_id), None);
    assert_eq!(tree.parent_id(child_id).unwrap(), parent_id);
    assert_eq!(tree.children_ids(parent_id).unwrap(), &[child_id]);
    let view = tree.state_view::<i32>();
    assert_eq!(*view.get(parent_id).unwrap(), 1);
    assert_eq!(*view.get(child_id).unwrap(), 0);
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

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 4);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(child), Some(1));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(child).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[before, child, after]);
    let view = tree.state_view::<i32>();
    assert_eq!(*view.get(parent).unwrap(), 0);
    assert_eq!(*view.get(before).unwrap(), 1);
    assert_eq!(*view.get(child).unwrap(), 2);
    assert_eq!(*view.get(after).unwrap(), 3);
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

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 4);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(child), Some(1));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(child).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[before, child, after]);
    {
        let view = tree.state_view::<i32>();
        assert_eq!(*view.get(parent).unwrap(), 0);
        assert_eq!(*view.get(before).unwrap(), 1);
        assert_eq!(*view.get(child).unwrap(), 2);
        assert_eq!(*view.get(after).unwrap(), 3);
    }

    tree.remove(child);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 3);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[before, after]);
    {
        let view = tree.state_view::<i32>();
        assert_eq!(*view.get(parent).unwrap(), 0);
        assert_eq!(*view.get(before).unwrap(), 1);
        assert_eq!(view.get(child), None);
        assert_eq!(*view.get(after).unwrap(), 3);
    }

    tree.remove(before);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 2);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[after]);
    {
        let view = tree.state_view::<i32>();
        assert_eq!(*view.get(parent).unwrap(), 0);
        assert_eq!(view.get(before), None);
        assert_eq!(*view.get(after).unwrap(), 3);
    }

    tree.remove(after);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 1);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.children_ids(parent).unwrap(), &[]);
    {
        let view = tree.state_view::<i32>();
        assert_eq!(*view.get(parent).unwrap(), 0);
        assert_eq!(view.get(after), None);
    }
}

#[test]
fn traverse_depth_first() {
    let mut tree = Tree::new();
    let parent = tree.root();
    tree.insert(parent, 0);
    let mut child1 = tree.create_node();
    child1.insert(1);
    let child1 = child1.id();
    tree.add_child(parent, child1);
    let mut grandchild1 = tree.create_node();
    grandchild1.insert(2);
    let grandchild1 = grandchild1.id();
    tree.add_child(child1, grandchild1);
    let mut child2 = tree.create_node();
    child2.insert(3);
    let child2 = child2.id();
    tree.add_child(parent, child2);
    let mut grandchild2 = tree.create_node();
    grandchild2.insert(4);
    let grandchild2 = grandchild2.id();
    tree.add_child(child2, grandchild2);

    let view = tree.state_view::<i32>();
    let mut node_count = 0;
    view.traverse_depth_first(move |node| {
        assert_eq!(*node, node_count);
        node_count += 1;
    });
}

#[test]
fn get_node_children_mut() {
    let mut tree = Tree::new();
    let parent = tree.root();
    tree.insert(parent, 0);
    let mut child1 = tree.create_node();
    child1.insert(1);
    let child1 = child1.id();
    tree.add_child(parent, child1);
    let mut child2 = tree.create_node();
    child2.insert(2);
    let child2 = child2.id();
    tree.add_child(parent, child2);
    let mut child3 = tree.create_node();
    child3.insert(3);
    let child3 = child3.id();
    tree.add_child(parent, child3);

    let mut view = tree.state_view_mut::<i32>();
    let (parent, children) = view.parent_child_mut(parent).unwrap();
    println!("Parent: {:#?}", parent);
    println!("Children: {:#?}", children);
    for (i, child) in children.into_iter().enumerate() {
        assert_eq!(*child, i as i32 + 1);
    }
}

#[derive(Debug)]
struct SlabStorage<T> {
    data: Vec<Option<T>>,
}

impl<T> Default for SlabStorage<T> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}

trait AnySlabStorageImpl: Any {
    fn remove(&mut self, id: usize);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> SlabStorage<T> {
    fn get(&self, id: usize) -> Option<&T> {
        self.data.get(id).and_then(|x| x.as_ref())
    }

    unsafe fn get_unchecked(&self, id: usize) -> &T {
        self.data.get_unchecked(id).as_ref().unwrap_unchecked()
    }

    fn get_mut(&mut self, id: usize) -> Option<&mut T> {
        self.data.get_mut(id).and_then(|x| x.as_mut())
    }

    unsafe fn get_unchecked_mut(&mut self, id: usize) -> &mut T {
        self.data.get_unchecked_mut(id).as_mut().unwrap_unchecked()
    }

    fn insert(&mut self, id: usize, value: T) {
        if id >= self.data.len() {
            self.data.resize_with(id + 1, || None);
        }
        self.data[id] = Some(value);
    }

    fn get2_mut(&mut self, id1: usize, id2: usize) -> Option<(&mut T, &mut T)> {
        assert!(id1 != id2);
        let ptr = self.data.as_mut_ptr();
        let first = unsafe { &mut *ptr.add(id1) };
        let second = unsafe { &mut *ptr.add(id2) };
        if let (Some(first), Some(second)) = (first, second) {
            Some((first, second))
        } else {
            None
        }
    }

    unsafe fn get_many_mut_unchecked(
        &mut self,
        ids: impl Iterator<Item = usize>,
    ) -> Option<Vec<&mut T>> {
        let ptr = self.data.as_mut_ptr();
        let mut result = Vec::new();
        for id in ids {
            if id >= self.data.len() {
                return None;
            }
            let item = unsafe { &mut *ptr.add(id) };
            if let Some(item) = item {
                result.push(item);
            } else {
                return None;
            }
        }
        Some(result)
    }
}

impl<T: 'static> AnySlabStorageImpl for SlabStorage<T> {
    fn remove(&mut self, id: usize) {
        self.data[id].take();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

struct AnySlab {
    data: hashbrown::HashMap<
        TypeId,
        RwLock<Box<dyn AnySlabStorageImpl>>,
        BuildHasherDefault<FxHasher>,
    >,
    free: VecDeque<usize>,
    len: usize,
}

impl Debug for AnySlab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnySlab")
            .field("data", &self.data.keys().collect::<Vec<_>>())
            .field("free", &self.free)
            .field("len", &self.len)
            .finish()
    }
}

impl Default for AnySlab {
    fn default() -> Self {
        Self::new()
    }
}

impl AnySlab {
    fn new() -> Self {
        Self {
            data: Default::default(),
            free: VecDeque::new(),
            len: 0,
        }
    }

    fn try_get_slab<T: Any>(&self) -> Option<MappedRwLockReadGuard<'_, SlabStorage<T>>> {
        self.data
            .get(&TypeId::of::<T>())
            .map(|x| RwLockReadGuard::map(x.read(), |x| x.as_any().downcast_ref().unwrap()))
    }

    fn get_slab<T: Any>(&self) -> MappedRwLockReadGuard<'_, SlabStorage<T>> {
        self.try_get_slab().unwrap()
    }

    fn try_get_slab_mut<T: Any>(&self) -> Option<MappedRwLockWriteGuard<'_, SlabStorage<T>>> {
        self.data
            .get(&TypeId::of::<T>())
            .map(|x| RwLockWriteGuard::map(x.write(), |x| x.as_any_mut().downcast_mut().unwrap()))
    }

    fn get_slab_mut<T: Any>(&self) -> MappedRwLockWriteGuard<'_, SlabStorage<T>> {
        self.try_get_slab_mut().unwrap()
    }

    fn get_or_insert_slab_mut<T: Any>(&mut self) -> MappedRwLockWriteGuard<'_, SlabStorage<T>> {
        RwLockWriteGuard::map(
            self.data
                .entry(TypeId::of::<T>())
                .or_insert_with(|| RwLock::new(Box::new(SlabStorage::<T>::default())))
                .write(),
            |x| x.as_any_mut().downcast_mut().unwrap(),
        )
    }

    fn insert(&mut self) -> Entry<'_> {
        let id = if let Some(id) = self.free.pop_front() {
            id
        } else {
            let id = self.len;
            self.len += 1;
            id
        };
        Entry { id, inner: self }
    }

    fn add<T: Any>(&mut self, id: usize, value: T) {
        self.get_or_insert_slab_mut().insert(id, value);
    }

    fn remove(&mut self, id: usize) {
        for slab in self.data.values_mut() {
            slab.write().remove(id);
        }
        self.free.push_back(id);
    }

    fn len(&self) -> usize {
        self.len - self.free.len()
    }
}

pub struct Entry<'a> {
    id: usize,
    inner: &'a mut AnySlab,
}

impl Entry<'_> {
    pub fn insert<T: Any>(&mut self, value: T) {
        self.inner
            .get_or_insert_slab_mut::<T>()
            .insert(self.id, value);
    }

    pub fn remove(self) {
        self.inner.remove(self.id);
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

#[test]
fn remove() {
    let mut slab = AnySlab::new();
    let mut node1 = slab.insert();
    node1.insert(0i32);
    let node1_id = node1.id();
    let mut node2 = slab.insert();
    node2.insert(0i32);

    assert_eq!(slab.len(), 2);

    slab.remove(node1_id);

    assert!(slab.get_slab::<i32>().get(node1_id).is_none());
    assert_eq!(slab.len(), 1);
}

#[test]
fn get_many_mut_unchecked() {
    let mut slab = AnySlab::new();
    let mut parent = slab.insert();
    parent.insert(0i32);
    let parent = parent.id();
    let mut child = slab.insert();
    child.insert(1i32);
    let child = child.id();
    let mut grandchild = slab.insert();
    grandchild.insert(2i32);
    let grandchild = grandchild.id();
    assert_eq!(slab.len(), 3);
    println!("ids: {:#?}", [parent, child, grandchild]);

    {
        let mut i32_slab = slab.get_slab_mut::<i32>();
        let all =
            unsafe { i32_slab.get_many_mut_unchecked([parent, child, grandchild].into_iter()) }
                .unwrap();
        assert_eq!(all, [&mut 0, &mut 1, &mut 2]);
    }

    {
        let mut i32_slab = slab.get_slab_mut::<i32>();
        assert!(unsafe { i32_slab.get_many_mut_unchecked([3, 100].into_iter()) }.is_none())
    }
}

#[test]
fn get_many_many_mut_unchecked() {
    let mut slab = AnySlab::new();
    let mut parent = slab.insert();
    parent.insert(0i32);
    parent.insert("0");
    let parent = parent.id();
    let mut child = slab.insert();
    child.insert(1i32);
    child.insert("1");
    let child = child.id();
    let mut grandchild = slab.insert();
    grandchild.insert(2i32);
    grandchild.insert("2");
    let grandchild = grandchild.id();
    println!("ids: {:#?}", [parent, child, grandchild]);

    println!("slab: {:#?}", slab);

    let mut num_entries = slab.get_slab_mut::<i32>();
    let mut str_entries = slab.get_slab_mut::<&str>();

    let all_num = unsafe {
        num_entries
            .as_any_mut()
            .downcast_mut::<SlabStorage<i32>>()
            .unwrap()
            .get_many_mut_unchecked([parent, child, grandchild].into_iter())
    }
    .unwrap();

    assert_eq!(all_num, [&mut 0, &mut 1, &mut 2]);

    let all_str = unsafe {
        str_entries
            .as_any_mut()
            .downcast_mut::<SlabStorage<&str>>()
            .unwrap()
            .get_many_mut_unchecked([parent, child, grandchild].into_iter())
    }
    .unwrap();

    assert_eq!(all_str, [&mut "0", &mut "1", &mut "2"]);
}
