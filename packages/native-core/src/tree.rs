use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use rustc_hash::{FxHashMap, FxHasher};
use std::any::{Any, TypeId};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::hash::BuildHasherDefault;

use crate::{AnyMapLike, Dependancy};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Hash)]
pub struct NodeId(pub usize);

#[derive(PartialEq, Eq, Clone, Debug)]
pub(crate) struct Node {
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    height: u16,
}

#[derive(Debug)]
pub(crate) struct Tree {
    nodes: AnySlab,
    root: NodeId,
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

    fn node_slab(&self) -> &SlabStorage<Node> {
        self.nodes.read_slab()
    }

    pub fn get_node_data(&self, id: NodeId) -> &Node {
        self.node_slab().get(id).unwrap()
    }

    fn node_slab_mut(&mut self) -> &mut SlabStorage<Node> {
        self.nodes.write_slab()
    }

    pub fn get_node_data_mut(&mut self, id: NodeId) -> &mut Node {
        self.node_slab_mut().get_mut(id).unwrap()
    }

    pub fn remove(&mut self, id: NodeId) {
        fn recurse(tree: &mut Tree, id: NodeId) {
            let children = { tree.get_node_data(id).children.clone() };
            for child in children {
                recurse(tree, child);
            }

            tree.nodes.remove(id);
        }
        {
            let node_data_mut = self.node_slab_mut();
            if let Some(parent) = node_data_mut.get(id).unwrap().parent {
                let parent = node_data_mut.get_mut(parent).unwrap();
                parent.children.retain(|&child| child != id);
            }
        }

        recurse(self, id);
    }

    fn set_height(&mut self, node: NodeId, height: u16) {
        let children = {
            let mut node = self.get_node_data_mut(node);
            node.height = height;
            node.children.clone()
        };
        for child in children {
            self.set_height(child, height + 1);
        }
    }

    pub fn create_node(&mut self) -> EntryBuilder<'_> {
        let mut node = self.nodes.insert();
        node.insert(Node {
            parent: None,
            children: Vec::new(),
            height: 0,
        });
        node
    }

    pub fn add_child(&mut self, parent: NodeId, new: NodeId) {
        let node_state = self.node_slab_mut();
        node_state.get_mut(new).unwrap().parent = Some(parent);
        let parent = node_state.get_mut(parent).unwrap();
        parent.children.push(new);
        let height = parent.height + 1;
        self.set_height(new, height);
    }

    pub fn replace(&mut self, old_id: NodeId, new_id: NodeId) {
        {
            let node_state = self.node_slab_mut();
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
                self.set_height(new_id, height);
            }
        }
        // remove the old node
        self.remove(old_id);
    }

    pub fn insert_before(&mut self, old_id: NodeId, new_id: NodeId) {
        let node_state = self.node_slab_mut();
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
        self.set_height(new_id, height);
    }

    pub fn insert_after(&mut self, old_id: NodeId, new_id: NodeId) {
        let node_state = self.node_slab_mut();
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
        self.set_height(new_id, height);
    }

    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    pub fn dynamically_borrowed(&mut self) -> DynamicallyBorrowedTree<'_> {
        DynamicallyBorrowedTree {
            nodes: self.nodes.dynamically_borrowed(),
        }
    }

    pub fn read<T: Any>(&self, id: NodeId) -> Option<&T> {
        self.nodes.read_slab().get(id)
    }

    pub fn write<T: Any>(&mut self, id: NodeId) -> Option<&mut T> {
        self.nodes.write_slab().get_mut(id)
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains(id)
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn parent_id(&self, id: NodeId) -> Option<NodeId> {
        self.get_node_data(id).parent
    }

    pub fn children_ids(&self, id: NodeId) -> Option<Vec<NodeId>> {
        Some(self.get_node_data(id).children.clone())
    }

    pub fn height(&self, id: NodeId) -> Option<u16> {
        Some(self.get_node_data(id).height)
    }
}

pub(crate) struct DynamicallyBorrowedTree<'a> {
    nodes: DynamiclyBorrowedAnySlabView<'a>,
}

impl<'a> DynamicallyBorrowedTree<'a> {
    pub fn view(
        &self,
        immutable: impl IntoIterator<Item = TypeId>,
        mutable: impl IntoIterator<Item = TypeId>,
    ) -> TreeStateView<'_, 'a> {
        let nodes_data = self.node_slab();
        let mut nodes = FxHashMap::default();
        for id in immutable {
            nodes.insert(id, MaybeRead::Read(self.nodes.get_slab(id).unwrap()));
        }
        for id in mutable {
            nodes.insert(id, MaybeRead::Write(self.nodes.get_slab_mut(id).unwrap()));
        }

        TreeStateView { nodes_data, nodes }
    }

    fn node_slab(&self) -> MappedRwLockReadGuard<SlabStorage<Node>> {
        RwLockReadGuard::map(self.nodes.get_slab(TypeId::of::<Node>()).unwrap(), |slab| {
            slab.as_any().downcast_ref().unwrap()
        })
    }
}

enum MaybeRead<'a, 'b> {
    Read(RwLockReadGuard<'a, &'b mut dyn AnySlabStorageImpl>),
    Write(RwLockWriteGuard<'a, &'b mut dyn AnySlabStorageImpl>),
}

#[derive(Clone, Copy)]
pub struct TreeStateViewEntry<'a, 'b> {
    view: &'a TreeStateView<'a, 'b>,
    id: NodeId,
}

impl<'a, 'b> AnyMapLike<'a> for TreeStateViewEntry<'a, 'b> {
    fn get<T: Any>(self) -> Option<&'a T> {
        self.view.get_slab().and_then(|slab| slab.get(self.id))
    }
}

pub struct TreeStateView<'a, 'b> {
    nodes_data: MappedRwLockReadGuard<'a, SlabStorage<Node>>,
    nodes: FxHashMap<TypeId, MaybeRead<'a, 'b>>,
}

impl<'a, 'b> TreeStateView<'a, 'b> {
    fn get_slab<T: Any>(&self) -> Option<&SlabStorage<T>> {
        self.nodes
            .get(&TypeId::of::<T>())
            .and_then(|gaurd| match gaurd {
                MaybeRead::Read(gaurd) => gaurd.as_any().downcast_ref::<SlabStorage<T>>(),
                MaybeRead::Write(gaurd) => gaurd.as_any().downcast_ref::<SlabStorage<T>>(),
            })
    }

    fn get_slab_mut<T: Any>(&mut self) -> Option<&mut SlabStorage<T>> {
        self.nodes
            .get_mut(&TypeId::of::<T>())
            .and_then(|gaurd| match gaurd {
                MaybeRead::Read(_gaurd) => None,
                MaybeRead::Write(gaurd) => gaurd.as_any_mut().downcast_mut::<SlabStorage<T>>(),
            })
    }

    pub fn children_ids(&self, id: NodeId) -> Option<Vec<NodeId>> {
        self.nodes_data.get(id).map(|node| node.children.clone())
    }

    pub fn parent_id(&self, id: NodeId) -> Option<NodeId> {
        self.nodes_data.get(id).and_then(|node| node.parent)
    }

    pub fn height(&self, id: NodeId) -> Option<u16> {
        self.nodes_data.get(id).map(|n| n.height)
    }

    pub fn get<T: Dependancy>(&self, id: NodeId) -> Option<T::ElementBorrowed<'_>> {
        T::borrow_elements_from(self.entry(id))
    }

    pub fn get_single<T: Any>(&self, id: NodeId) -> Option<&T> {
        self.get_slab().and_then(|slab| slab.get(id))
    }

    pub fn get_mut<T: Any>(&mut self, id: NodeId) -> Option<&mut T> {
        self.get_slab_mut().and_then(|slab| slab.get_mut(id))
    }

    pub fn entry(&self, id: NodeId) -> TreeStateViewEntry<'_, 'b> {
        TreeStateViewEntry { view: self, id }
    }

    pub fn children<T: Dependancy>(&self, id: NodeId) -> Option<Vec<T::ElementBorrowed<'_>>> {
        let ids = self.children_ids(id);
        ids.and_then(|ids| {
            ids.iter()
                .map(|id| T::borrow_elements_from(self.entry(*id)))
                .collect()
        })
    }

    pub fn parent<T: Dependancy>(&self, id: NodeId) -> Option<T::ElementBorrowed<'_>> {
        T::borrow_elements_from(self.entry(id))
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

    assert_eq!(*tree.read::<i32>(parent_id).unwrap(), 1);
    assert_eq!(*tree.read::<i32>(child_id).unwrap(), 0);
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

    assert_eq!(*tree.read::<i32>(parent).unwrap(), 0);
    assert_eq!(*tree.read::<i32>(before).unwrap(), 1);
    assert_eq!(*tree.read::<i32>(child).unwrap(), 2);
    assert_eq!(*tree.read::<i32>(after).unwrap(), 3);
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

    assert_eq!(*tree.read::<i32>(parent).unwrap(), 0);
    assert_eq!(*tree.read::<i32>(before).unwrap(), 1);
    assert_eq!(*tree.read::<i32>(child).unwrap(), 2);
    assert_eq!(*tree.read::<i32>(after).unwrap(), 3);

    tree.remove(child);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 3);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(before), Some(1));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(before).unwrap(), parent);
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[before, after]);

    assert_eq!(*tree.read::<i32>(parent).unwrap(), 0);
    assert_eq!(*tree.read::<i32>(before).unwrap(), 1);
    assert_eq!(tree.read::<i32>(child), None);
    assert_eq!(*tree.read::<i32>(after).unwrap(), 3);

    tree.remove(before);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 2);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.height(after), Some(1));
    assert_eq!(tree.parent_id(after).unwrap(), parent);
    assert_eq!(tree.children_ids(parent).unwrap(), &[after]);

    assert_eq!(*tree.read::<i32>(parent).unwrap(), 0);
    assert_eq!(tree.read::<i32>(before), None);
    assert_eq!(*tree.read::<i32>(after).unwrap(), 3);

    tree.remove(after);

    println!("Tree: {:#?}", tree);
    assert_eq!(tree.size(), 1);
    assert_eq!(tree.height(parent), Some(0));
    assert_eq!(tree.children_ids(parent).unwrap(), &[]);

    assert_eq!(*tree.read::<i32>(parent).unwrap(), 0);
    assert_eq!(tree.read::<i32>(after), None);
}

#[derive(Debug)]
pub(crate) struct SlabStorage<T> {
    data: Vec<Option<T>>,
}

impl<T> Default for SlabStorage<T> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}

trait AnySlabStorageImpl: Any + Send + Sync {
    fn remove(&mut self, id: NodeId);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> SlabStorage<T> {
    fn get(&self, id: NodeId) -> Option<&T> {
        self.data.get(id.0).and_then(|x| x.as_ref())
    }

    fn get_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.data.get_mut(id.0).and_then(|x| x.as_mut())
    }

    fn insert(&mut self, id: NodeId, value: T) {
        let idx = id.0;
        if idx >= self.data.len() {
            self.data.resize_with(idx + 1, || None);
        }
        self.data[idx] = Some(value);
    }
}

impl<T: 'static + Send + Sync> AnySlabStorageImpl for SlabStorage<T> {
    fn remove(&mut self, id: NodeId) {
        self.data[id.0].take();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub(crate) struct DynamiclyBorrowedAnySlabView<'a> {
    data: hashbrown::HashMap<
        TypeId,
        RwLock<&'a mut dyn AnySlabStorageImpl>,
        BuildHasherDefault<FxHasher>,
    >,
}

impl<'a> DynamiclyBorrowedAnySlabView<'a> {
    fn get_slab<'b>(
        &'b self,
        type_id: TypeId,
    ) -> Option<RwLockReadGuard<'b, &'a mut dyn AnySlabStorageImpl>> {
        self.data.get(&type_id).map(|x| x.read())
    }

    fn get_slab_mut<'b>(
        &'b self,
        type_id: TypeId,
    ) -> Option<RwLockWriteGuard<'b, &'a mut dyn AnySlabStorageImpl>> {
        self.data.get(&type_id).map(|x| x.write())
    }
}

pub(crate) struct AnySlab {
    data: hashbrown::HashMap<TypeId, Box<dyn AnySlabStorageImpl>, BuildHasherDefault<FxHasher>>,
    filled: Vec<bool>,
    free: VecDeque<NodeId>,
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
            filled: Default::default(),
            free: VecDeque::new(),
            len: 0,
        }
    }

    fn try_read_slab<T: Any>(&self) -> Option<&SlabStorage<T>> {
        self.data
            .get(&TypeId::of::<T>())
            .map(|x| x.as_any().downcast_ref().unwrap())
    }

    fn read_slab<T: Any>(&self) -> &SlabStorage<T> {
        self.try_read_slab().unwrap()
    }

    fn try_write_slab<T: Any>(&mut self) -> Option<&mut SlabStorage<T>> {
        self.data
            .get_mut(&TypeId::of::<T>())
            .map(|x| x.as_any_mut().downcast_mut().unwrap())
    }

    fn write_slab<T: Any>(&mut self) -> &mut SlabStorage<T> {
        self.try_write_slab().unwrap()
    }

    fn get_or_insert_slab_mut<T: Any + Send + Sync>(&mut self) -> &mut SlabStorage<T> {
        self.data
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::<SlabStorage<T>>::default())
            .as_any_mut()
            .downcast_mut()
            .unwrap()
    }

    fn insert(&mut self) -> EntryBuilder<'_> {
        let id = if let Some(id) = self.free.pop_front() {
            self.filled[id.0] = true;
            id
        } else {
            let id = self.len;
            self.filled.push(true);
            self.len += 1;
            NodeId(id)
        };
        EntryBuilder { id, inner: self }
    }

    fn remove(&mut self, id: NodeId) {
        for slab in self.data.values_mut() {
            slab.remove(id);
        }
        self.filled[id.0] = true;
        self.free.push_back(id);
    }

    fn len(&self) -> usize {
        self.len - self.free.len()
    }

    fn contains(&self, id: NodeId) -> bool {
        self.filled.get(id.0).copied().unwrap_or_default()
    }

    fn dynamically_borrowed(&mut self) -> DynamiclyBorrowedAnySlabView<'_> {
        DynamiclyBorrowedAnySlabView {
            data: self
                .data
                .iter_mut()
                .map(|(k, v)| (*k, RwLock::new(&mut **v)))
                .collect(),
        }
    }
}

pub struct EntryBuilder<'a> {
    id: NodeId,
    inner: &'a mut AnySlab,
}

impl EntryBuilder<'_> {
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) {
        self.inner
            .get_or_insert_slab_mut::<T>()
            .insert(self.id, value);
    }

    pub fn get<T: Any>(&self) -> Option<&T> {
        self.inner.read_slab().get(self.id)
    }

    pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.inner.write_slab().get_mut(self.id)
    }

    pub fn remove(self) {
        self.inner.remove(self.id);
    }

    pub fn id(&self) -> NodeId {
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

    assert!(slab.read_slab::<i32>().get(node1_id).is_none());
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
        let i32_slab = slab.write_slab::<i32>();
        let all =
            unsafe { i32_slab.get_many_mut_unchecked([parent, child, grandchild].into_iter()) }
                .unwrap();
        assert_eq!(all, [&mut 0, &mut 1, &mut 2]);
    }

    {
        let i32_slab = slab.write_slab::<i32>();
        assert!(
            unsafe { i32_slab.get_many_mut_unchecked([NodeId(3), NodeId(100)].into_iter()) }
                .is_none()
        )
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

    let num_entries = slab.write_slab::<i32>();

    let all_num = unsafe {
        num_entries
            .as_any_mut()
            .downcast_mut::<SlabStorage<i32>>()
            .unwrap()
            .get_many_mut_unchecked([parent, child, grandchild].into_iter())
    }
    .unwrap();

    assert_eq!(all_num, [&mut 0, &mut 1, &mut 2]);
    let str_entries = slab.write_slab::<&str>();

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
