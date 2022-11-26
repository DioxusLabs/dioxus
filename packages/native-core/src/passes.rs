use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use std::collections::BTreeMap;
use std::hash::BuildHasherDefault;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::tree::{NodeId, TreeView};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirtyNodes {
    map: BTreeMap<u16, FxHashSet<NodeId>>,
}

impl DirtyNodes {
    pub fn insert(&mut self, depth: u16, node_id: NodeId) {
        self.map
            .entry(depth)
            .or_insert_with(FxHashSet::default)
            .insert(node_id);
    }

    fn pop_front(&mut self) -> Option<NodeId> {
        let (&depth, values) = self.map.iter_mut().next()?;
        let key = *values.iter().next()?;
        let node_id = values.take(&key)?;
        if values.is_empty() {
            self.map.remove(&depth);
        }
        Some(node_id)
    }

    fn pop_back(&mut self) -> Option<NodeId> {
        let (&depth, values) = self.map.iter_mut().rev().next()?;
        let key = *values.iter().next()?;
        let node_id = values.take(&key)?;
        if values.is_empty() {
            self.map.remove(&depth);
        }
        Some(node_id)
    }
}

#[test]
fn dirty_nodes() {
    let mut dirty_nodes = DirtyNodes::default();

    dirty_nodes.insert(1, NodeId(1));
    dirty_nodes.insert(0, NodeId(0));
    dirty_nodes.insert(2, NodeId(3));
    dirty_nodes.insert(1, NodeId(2));

    assert_eq!(dirty_nodes.pop_front(), Some(NodeId(0)));
    assert!(matches!(dirty_nodes.pop_front(), Some(NodeId(1 | 2))));
    assert!(matches!(dirty_nodes.pop_front(), Some(NodeId(1 | 2))));
    assert_eq!(dirty_nodes.pop_front(), Some(NodeId(3)));
}

type FxDashMap<K, V> = dashmap::DashMap<K, V, BuildHasherDefault<FxHasher>>;

#[derive(Default)]
pub struct DirtyNodeStates {
    dirty: FxDashMap<NodeId, Vec<AtomicU64>>,
}

impl DirtyNodeStates {
    pub fn new(starting_nodes: FxHashMap<NodeId, FxHashSet<PassId>>) -> Self {
        let this = Self::default();
        for (node, nodes) in starting_nodes {
            for pass_id in nodes {
                this.insert(pass_id, node);
            }
        }
        this
    }

    pub fn insert(&self, pass_id: PassId, node_id: NodeId) {
        let pass_id = pass_id.0;
        let index = pass_id / 64;
        let bit = pass_id % 64;
        let encoded = 1 << bit;
        if let Some(dirty) = self.dirty.get(&node_id) {
            if let Some(atomic) = dirty.get(index as usize) {
                atomic.fetch_or(encoded, Ordering::Relaxed);
            } else {
                drop(dirty);
                let mut write = self.dirty.get_mut(&node_id).unwrap();
                write.resize_with(index as usize + 1, || AtomicU64::new(0));
                write[index as usize].fetch_or(encoded, Ordering::Relaxed);
            }
        } else {
            let mut v = Vec::with_capacity(index as usize + 1);
            v.resize_with(index as usize + 1, || AtomicU64::new(0));
            v[index as usize].fetch_or(encoded, Ordering::Relaxed);
            self.dirty.insert(node_id, v);
        }
    }

    fn all_dirty<T>(&self, pass_id: PassId, dirty_nodes: &mut DirtyNodes, tree: &impl TreeView<T>) {
        let pass_id = pass_id.0;
        let index = pass_id / 64;
        let bit = pass_id % 64;
        let encoded = 1 << bit;
        for entry in self.dirty.iter() {
            let node_id = entry.key();
            let dirty = entry.value();
            if let Some(atomic) = dirty.get(index as usize) {
                if atomic.load(Ordering::Relaxed) & encoded != 0 {
                    dirty_nodes.insert(tree.height(*node_id).unwrap(), *node_id);
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct PassId(u64);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
pub struct MemberMask(u64);

impl MemberMask {
    pub fn overlaps(&self, other: Self) -> bool {
        (*self & other).0 != 0
    }
}

impl BitAndAssign for MemberMask {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitAnd for MemberMask {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        MemberMask(self.0 & rhs.0)
    }
}

impl BitOrAssign for MemberMask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitOr for MemberMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

pub struct PassReturn {
    progress: bool,
    mark_dirty: bool,
}

pub trait Pass {
    fn pass_id(&self) -> PassId;
    fn dependancies(&self) -> &'static [PassId];
    fn dependants(&self) -> &'static [PassId];
    fn mask(&self) -> MemberMask;
}

pub trait UpwardPass<T>: Pass {
    fn pass<'a>(&self, node: &mut T, children: &mut dyn Iterator<Item = &'a mut T>) -> PassReturn;
}

fn resolve_upward_pass<T, P: UpwardPass<T> + ?Sized>(
    tree: &mut impl TreeView<T>,
    pass: &P,
    mut dirty: DirtyNodes,
    dirty_states: &DirtyNodeStates,
) {
    while let Some(id) = dirty.pop_back() {
        let (node, mut children) = tree.parent_child_mut(id).unwrap();
        let result = pass.pass(node, &mut children);
        drop(children);
        if result.progress || result.mark_dirty {
            if let Some(id) = tree.parent_id(id) {
                if result.mark_dirty {
                    for dependant in pass.dependants() {
                        dirty_states.insert(*dependant, id);
                    }
                }
                if result.progress {
                    let height = tree.height(id).unwrap();
                    dirty.insert(height, id);
                }
            }
        }
    }
}

pub trait DownwardPass<T>: Pass {
    fn pass(&self, node: &mut T, parent: Option<&mut T>) -> PassReturn;
}

fn resolve_downward_pass<T, P: DownwardPass<T> + ?Sized>(
    tree: &mut impl TreeView<T>,
    pass: &P,
    mut dirty: DirtyNodes,
    dirty_states: &DirtyNodeStates,
) {
    while let Some(id) = dirty.pop_front() {
        let (node, parent) = tree.node_parent_mut(id).unwrap();
        let result = pass.pass(node, parent);
        if result.mark_dirty || result.progress {
            for id in tree.children_ids(id).unwrap() {
                if result.mark_dirty {
                    for dependant in pass.dependants() {
                        dirty_states.insert(*dependant, *id);
                    }
                }
                if result.progress {
                    let height = tree.height(*id).unwrap();
                    dirty.insert(height, *id);
                }
            }
        }
    }
}

pub trait NodePass<T>: Pass {
    fn pass(&self, node: &mut T) -> bool;
}

fn resolve_node_pass<T, P: NodePass<T> + ?Sized>(
    tree: &mut impl TreeView<T>,
    pass: &P,
    mut dirty: DirtyNodes,
    dirty_states: &DirtyNodeStates,
) {
    while let Some(id) = dirty.pop_back() {
        let node = tree.get_mut(id).unwrap();
        if pass.pass(node) {
            for dependant in pass.dependants() {
                dirty_states.insert(*dependant, id);
            }
        }
    }
}

pub enum AnyPass<T> {
    Upward(Box<dyn UpwardPass<T> + Send + Sync>),
    Downward(Box<dyn DownwardPass<T> + Send + Sync>),
    Node(Box<dyn NodePass<T> + Send + Sync>),
}

impl<T> AnyPass<T> {
    fn pass_id(&self) -> PassId {
        match self {
            Self::Upward(pass) => pass.pass_id(),
            Self::Downward(pass) => pass.pass_id(),
            Self::Node(pass) => pass.pass_id(),
        }
    }

    fn dependancies(&self) -> &'static [PassId] {
        match self {
            Self::Upward(pass) => pass.dependancies(),
            Self::Downward(pass) => pass.dependancies(),
            Self::Node(pass) => pass.dependancies(),
        }
    }

    fn mask(&self) -> MemberMask {
        match self {
            Self::Upward(pass) => pass.mask(),
            Self::Downward(pass) => pass.mask(),
            Self::Node(pass) => pass.mask(),
        }
    }

    fn resolve(
        &self,
        tree: &mut impl TreeView<T>,
        dirty: DirtyNodes,
        dirty_states: &DirtyNodeStates,
    ) {
        match self {
            Self::Downward(pass) => resolve_downward_pass(tree, pass.as_ref(), dirty, dirty_states),
            Self::Upward(pass) => resolve_upward_pass(tree, pass.as_ref(), dirty, dirty_states),
            Self::Node(pass) => resolve_node_pass(tree, pass.as_ref(), dirty, dirty_states),
        }
    }
}

struct RawPointer<T>(*mut T);
unsafe impl<T> Send for RawPointer<T> {}
unsafe impl<T> Sync for RawPointer<T> {}

pub fn resolve_passes<T, Tr: TreeView<T>>(
    tree: &mut Tr,
    dirty_nodes: DirtyNodeStates,
    mut passes: Vec<AnyPass<T>>,
) {
    let dirty_states = Arc::new(dirty_nodes);
    let mut resolved_passes: FxHashSet<PassId> = FxHashSet::default();
    let mut resolving = Vec::new();
    while !passes.is_empty() {
        let mut currently_borrowed = MemberMask::default();
        std::thread::scope(|s| {
            let mut i = 0;
            while i < passes.len() {
                let pass = &passes[i];
                let pass_id = pass.pass_id();
                let pass_mask = pass.mask();
                if pass
                    .dependancies()
                    .iter()
                    .all(|d| resolved_passes.contains(d))
                    && !pass_mask.overlaps(currently_borrowed)
                {
                    let pass = passes.remove(i);
                    resolving.push(pass_id);
                    currently_borrowed |= pass_mask;
                    let tree_mut = tree as *mut _;
                    let raw_ptr = RawPointer(tree_mut);
                    let dirty_states = dirty_states.clone();
                    s.spawn(move || unsafe {
                        // let tree_mut: &mut Tr = &mut *raw_ptr.0;
                        let raw = raw_ptr;
                        let tree_mut: &mut Tr = &mut *raw.0;
                        let mut dirty = DirtyNodes::default();
                        dirty_states.all_dirty(pass_id, &mut dirty, tree_mut);
                        pass.resolve(tree_mut, dirty, &dirty_states);
                    });
                } else {
                    i += 1;
                }
            }
            // all passes are resolved at the end of the scope
        });
        resolved_passes.extend(resolving.iter().copied());
        resolving.clear()
    }
}

#[test]
fn node_pass() {
    use crate::tree::{Tree, TreeLike};
    let mut tree = Tree::new(0);
    let parent = tree.root();
    let child1 = tree.create_node(1);
    tree.add_child(parent, child1);
    let grandchild1 = tree.create_node(3);
    tree.add_child(child1, grandchild1);
    let child2 = tree.create_node(2);
    tree.add_child(parent, child2);
    let grandchild2 = tree.create_node(4);
    tree.add_child(child2, grandchild2);
    println!("{:#?}", tree);

    struct AddPass;
    impl Pass for AddPass {
        fn pass_id(&self) -> PassId {
            PassId(0)
        }

        fn dependancies(&self) -> &'static [PassId] {
            &[]
        }

        fn dependants(&self) -> &'static [PassId] {
            &[]
        }

        fn mask(&self) -> MemberMask {
            MemberMask(1)
        }
    }

    impl NodePass<i32> for AddPass {
        fn pass(&self, node: &mut i32) -> bool {
            *node += 1;
            true
        }
    }

    let passes = vec![AnyPass::Node(Box::new(AddPass))];
    let dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
    dirty_nodes.insert(PassId(0), tree.root());
    resolve_passes(&mut tree, dirty_nodes, passes);

    assert_eq!(tree.get(tree.root()).unwrap(), &1);
}

#[test]
fn dependant_node_pass() {
    use crate::tree::{Tree, TreeLike};
    let mut tree = Tree::new(0);
    let parent = tree.root();
    let child1 = tree.create_node(1);
    tree.add_child(parent, child1);
    let grandchild1 = tree.create_node(3);
    tree.add_child(child1, grandchild1);
    let child2 = tree.create_node(2);
    tree.add_child(parent, child2);
    let grandchild2 = tree.create_node(4);
    tree.add_child(child2, grandchild2);

    struct AddPass;
    impl Pass for AddPass {
        fn pass_id(&self) -> PassId {
            PassId(0)
        }

        fn dependancies(&self) -> &'static [PassId] {
            &[PassId(1)]
        }

        fn dependants(&self) -> &'static [PassId] {
            &[]
        }

        fn mask(&self) -> MemberMask {
            MemberMask(1)
        }
    }

    impl NodePass<i32> for AddPass {
        fn pass(&self, node: &mut i32) -> bool {
            println!("AddPass: {}", node);
            *node += 1;
            true
        }
    }

    struct SubtractPass;

    impl Pass for SubtractPass {
        fn pass_id(&self) -> PassId {
            PassId(1)
        }

        fn dependancies(&self) -> &'static [PassId] {
            &[]
        }

        fn dependants(&self) -> &'static [PassId] {
            &[PassId(0)]
        }

        fn mask(&self) -> MemberMask {
            MemberMask(1)
        }
    }
    impl NodePass<i32> for SubtractPass {
        fn pass(&self, node: &mut i32) -> bool {
            println!("SubtractPass: {}", node);
            *node -= 1;
            true
        }
    }

    let passes = vec![
        AnyPass::Node(Box::new(AddPass)),
        AnyPass::Node(Box::new(SubtractPass)),
    ];
    let dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
    dirty_nodes.insert(PassId(1), tree.root());
    resolve_passes(&mut tree, dirty_nodes, passes);

    assert_eq!(*tree.get(tree.root()).unwrap(), 0);
}

#[test]
fn down_pass() {
    use crate::tree::{Tree, TreeLike};
    let mut tree = Tree::new(1);
    let parent = tree.root();
    let child1 = tree.create_node(1);
    tree.add_child(parent, child1);
    let grandchild1 = tree.create_node(1);
    tree.add_child(child1, grandchild1);
    let child2 = tree.create_node(1);
    tree.add_child(parent, child2);
    let grandchild2 = tree.create_node(1);
    tree.add_child(child2, grandchild2);

    struct AddPass;

    impl Pass for AddPass {
        fn pass_id(&self) -> PassId {
            PassId(0)
        }

        fn dependancies(&self) -> &'static [PassId] {
            &[]
        }

        fn dependants(&self) -> &'static [PassId] {
            &[]
        }

        fn mask(&self) -> MemberMask {
            MemberMask(1)
        }
    }
    impl DownwardPass<i32> for AddPass {
        fn pass(&self, node: &mut i32, parent: Option<&mut i32>) -> PassReturn {
            if let Some(parent) = parent {
                *node += *parent;
            }
            PassReturn {
                progress: true,
                mark_dirty: true,
            }
        }
    }

    let passes = vec![AnyPass::Downward(Box::new(AddPass))];
    let dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
    dirty_nodes.insert(PassId(0), tree.root());
    resolve_passes(&mut tree, dirty_nodes, passes);

    assert_eq!(tree.get(tree.root()).unwrap(), &1);
    assert_eq!(tree.get(child1).unwrap(), &2);
    assert_eq!(tree.get(grandchild1).unwrap(), &3);
    assert_eq!(tree.get(child2).unwrap(), &2);
    assert_eq!(tree.get(grandchild2).unwrap(), &3);
}

#[test]
fn dependant_down_pass() {
    use crate::tree::{Tree, TreeLike};
    // 0
    let mut tree = Tree::new(1);
    let parent = tree.root();
    // 1
    let child1 = tree.create_node(1);
    tree.add_child(parent, child1);
    // 2
    let grandchild1 = tree.create_node(1);
    tree.add_child(child1, grandchild1);
    // 3
    let child2 = tree.create_node(1);
    tree.add_child(parent, child2);
    // 4
    let grandchild2 = tree.create_node(1);
    tree.add_child(child2, grandchild2);

    struct AddPass;
    impl Pass for AddPass {
        fn pass_id(&self) -> PassId {
            PassId(0)
        }

        fn dependancies(&self) -> &'static [PassId] {
            &[PassId(1)]
        }

        fn dependants(&self) -> &'static [PassId] {
            &[]
        }

        fn mask(&self) -> MemberMask {
            MemberMask(1)
        }
    }
    impl DownwardPass<i32> for AddPass {
        fn pass(&self, node: &mut i32, parent: Option<&mut i32>) -> PassReturn {
            if let Some(parent) = parent {
                println!("AddPass: {} -> {}", node, *node + *parent);
                *node += *parent;
            } else {
                println!("AddPass: {}", node);
            }
            PassReturn {
                progress: true,
                mark_dirty: true,
            }
        }
    }

    struct SubtractPass;
    impl Pass for SubtractPass {
        fn pass_id(&self) -> PassId {
            PassId(1)
        }

        fn dependancies(&self) -> &'static [PassId] {
            &[]
        }

        fn dependants(&self) -> &'static [PassId] {
            &[PassId(0)]
        }

        fn mask(&self) -> MemberMask {
            MemberMask(1)
        }
    }
    impl DownwardPass<i32> for SubtractPass {
        fn pass(&self, node: &mut i32, parent: Option<&mut i32>) -> PassReturn {
            if let Some(parent) = parent {
                println!("SubtractPass: {} -> {}", node, *node - *parent);
                *node -= *parent;
            } else {
                println!("SubtractPass: {}", node);
            }
            PassReturn {
                progress: true,
                mark_dirty: true,
            }
        }
    }

    let passes = vec![
        AnyPass::Downward(Box::new(AddPass)),
        AnyPass::Downward(Box::new(SubtractPass)),
    ];
    let dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
    dirty_nodes.insert(PassId(1), tree.root());
    resolve_passes(&mut tree, dirty_nodes, passes);

    // Tree before:
    // 1=\
    //   1=\
    //     1
    //   1=\
    //     1
    // Tree after subtract:
    // 1=\
    //   0=\
    //     1
    //   0=\
    //     1
    // Tree after add:
    // 1=\
    //   1=\
    //     2
    //   1=\
    //     2
    assert_eq!(tree.get(tree.root()).unwrap(), &1);
    assert_eq!(tree.get(child1).unwrap(), &1);
    assert_eq!(tree.get(grandchild1).unwrap(), &2);
    assert_eq!(tree.get(child2).unwrap(), &1);
    assert_eq!(tree.get(grandchild2).unwrap(), &2);
}

#[test]
fn up_pass() {
    use crate::tree::{Tree, TreeLike};
    // Tree before:
    // 0=\
    //   0=\
    //     1
    //   0=\
    //     1
    // Tree after:
    // 2=\
    //   1=\
    //     1
    //   1=\
    //     1
    let mut tree = Tree::new(0);
    let parent = tree.root();
    let child1 = tree.create_node(0);
    tree.add_child(parent, child1);
    let grandchild1 = tree.create_node(1);
    tree.add_child(child1, grandchild1);
    let child2 = tree.create_node(0);
    tree.add_child(parent, child2);
    let grandchild2 = tree.create_node(1);
    tree.add_child(child2, grandchild2);

    struct AddPass;
    impl Pass for AddPass {
        fn pass_id(&self) -> PassId {
            PassId(0)
        }

        fn dependancies(&self) -> &'static [PassId] {
            &[]
        }

        fn dependants(&self) -> &'static [PassId] {
            &[]
        }

        fn mask(&self) -> MemberMask {
            MemberMask(1)
        }
    }
    impl UpwardPass<i32> for AddPass {
        fn pass<'a>(
            &self,
            node: &mut i32,
            children: &mut dyn Iterator<Item = &'a mut i32>,
        ) -> PassReturn {
            *node += children.map(|i| *i).sum::<i32>();
            PassReturn {
                progress: true,
                mark_dirty: true,
            }
        }
    }

    let passes = vec![AnyPass::Upward(Box::new(AddPass))];
    let dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
    dirty_nodes.insert(PassId(0), grandchild1);
    dirty_nodes.insert(PassId(0), grandchild2);
    resolve_passes(&mut tree, dirty_nodes, passes);

    assert_eq!(tree.get(tree.root()).unwrap(), &2);
    assert_eq!(tree.get(child1).unwrap(), &1);
    assert_eq!(tree.get(grandchild1).unwrap(), &1);
    assert_eq!(tree.get(child2).unwrap(), &1);
    assert_eq!(tree.get(grandchild2).unwrap(), &1);
}
