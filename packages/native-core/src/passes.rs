use crate::tree::{NodeId, TreeView};
use crate::{FxDashSet, SendAnyMap};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
use std::sync::Arc;

#[derive(Default)]
struct DirtyNodes {
    passes_dirty: Vec<u64>,
}

impl DirtyNodes {
    fn add_node(&mut self, node_id: NodeId) {
        let node_id = node_id.0;
        let index = node_id / 64;
        let bit = node_id % 64;
        let encoded = 1 << bit;
        if let Some(passes) = self.passes_dirty.get_mut(index) {
            *passes |= encoded;
        } else {
            self.passes_dirty.resize(index + 1, 0);
            self.passes_dirty[index] |= encoded;
        }
    }

    fn is_empty(&self) -> bool {
        self.passes_dirty.iter().all(|dirty| *dirty == 0)
    }

    fn pop(&mut self) -> Option<NodeId> {
        let index = self.passes_dirty.iter().position(|dirty| *dirty != 0)?;
        let passes = self.passes_dirty[index];
        let node_id = passes.trailing_zeros();
        let encoded = 1 << node_id;
        self.passes_dirty[index] &= !encoded;
        Some(NodeId((index * 64) + node_id as usize))
    }
}

#[derive(Default)]
pub struct DirtyNodeStates {
    dirty: BTreeMap<u16, FxHashMap<PassId, DirtyNodes>>,
}

impl DirtyNodeStates {
    pub fn insert(&mut self, pass_id: PassId, node_id: NodeId, height: u16) {
        if let Some(dirty) = self.dirty.get_mut(&height) {
            if let Some(entry) = dirty.get_mut(&pass_id) {
                entry.add_node(node_id);
            } else {
                let mut entry = DirtyNodes::default();
                entry.add_node(node_id);
                dirty.insert(pass_id, entry);
            }
        } else {
            let mut entry = DirtyNodes::default();
            entry.add_node(node_id);
            let mut hm = FxHashMap::default();
            hm.insert(pass_id, entry);
            self.dirty.insert(height, hm);
        }
    }

    fn pop_front(&mut self, pass_id: PassId) -> Option<(u16, NodeId)> {
        let (&height, values) = self
            .dirty
            .iter_mut()
            .find(|(_, values)| values.contains_key(&pass_id))?;
        let dirty = values.get_mut(&pass_id)?;
        let node_id = dirty.pop()?;
        if dirty.is_empty() {
            values.remove(&pass_id);
        }
        if values.is_empty() {
            self.dirty.remove(&height);
        }

        Some((height, node_id))
    }

    fn pop_back(&mut self, pass_id: PassId) -> Option<(u16, NodeId)> {
        let (&height, values) = self
            .dirty
            .iter_mut()
            .rev()
            .find(|(_, values)| values.contains_key(&pass_id))?;
        let dirty = values.get_mut(&pass_id)?;
        let node_id = dirty.pop()?;
        if dirty.is_empty() {
            values.remove(&pass_id);
        }
        if values.is_empty() {
            self.dirty.remove(&height);
        }

        Some((height, node_id))
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct PassId(pub u64);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
pub struct MemberMask(pub u64);

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
    pub progress: bool,
    pub mark_dirty: bool,
}

pub trait Pass {
    fn pass_id(&self) -> PassId;
    fn dependancies(&self) -> &'static [PassId];
    fn dependants(&self) -> &'static [PassId];
    fn mask(&self) -> MemberMask;
}

pub trait UpwardPass<T>: Pass {
    fn pass<'a>(
        &self,
        node: &mut T,
        children: &mut dyn Iterator<Item = &'a mut T>,
        ctx: &SendAnyMap,
    ) -> PassReturn;
}

fn resolve_upward_pass<T, P: UpwardPass<T> + ?Sized>(
    tree: &mut impl TreeView<T>,
    pass: &P,
    dirty_states: &mut DirtyNodeStates,
    nodes_updated: &FxDashSet<NodeId>,
    ctx: &SendAnyMap,
) {
    let pass_id = pass.pass_id();
    while let Some((height, id)) = dirty_states.pop_back(pass_id) {
        let (node, mut children) = tree.parent_child_mut(id).unwrap();
        let result = pass.pass(node, &mut children, ctx);
        drop(children);
        if result.progress || result.mark_dirty {
            nodes_updated.insert(id);
            if let Some(id) = tree.parent_id(id) {
                if result.mark_dirty {
                    for dependant in pass.dependants() {
                        dirty_states.insert(*dependant, id, height - 1);
                    }
                }
                if result.progress && height > 0 {
                    dirty_states.insert(pass_id, id, height - 1);
                }
            }
        }
    }
}

pub trait DownwardPass<T>: Pass {
    fn pass(&self, node: &mut T, parent: Option<&mut T>, ctx: &SendAnyMap) -> PassReturn;
}

fn resolve_downward_pass<T, P: DownwardPass<T> + ?Sized>(
    tree: &mut impl TreeView<T>,
    pass: &P,
    dirty_states: &mut DirtyNodeStates,
    nodes_updated: &FxDashSet<NodeId>,
    ctx: &SendAnyMap,
) {
    let pass_id = pass.pass_id();
    while let Some((height, id)) = dirty_states.pop_front(pass_id) {
        let (node, parent) = tree.node_parent_mut(id).unwrap();
        let result = pass.pass(node, parent, ctx);
        if result.mark_dirty {
            nodes_updated.insert(id);
        }
        if result.mark_dirty || result.progress {
            for id in tree.children_ids(id).unwrap() {
                if result.mark_dirty {
                    for dependant in pass.dependants() {
                        dirty_states.insert(*dependant, *id, height + 1);
                    }
                }
                if result.progress {
                    dirty_states.insert(pass_id, *id, height + 1);
                }
            }
        }
    }
}

pub trait NodePass<T>: Pass {
    fn pass(&self, node: &mut T, ctx: &SendAnyMap) -> bool;
}

fn resolve_node_pass<T, P: NodePass<T> + ?Sized>(
    tree: &mut impl TreeView<T>,
    pass: &P,
    dirty_states: &mut DirtyNodeStates,
    nodes_updated: &FxDashSet<NodeId>,
    ctx: &SendAnyMap,
) {
    let pass_id = pass.pass_id();
    while let Some((height, id)) = dirty_states.pop_back(pass_id) {
        let node = tree.get_mut(id).unwrap();
        if pass.pass(node, ctx) {
            nodes_updated.insert(id);
            for dependant in pass.dependants() {
                dirty_states.insert(*dependant, id, height);
            }
        }
    }
}

pub enum AnyPass<T: 'static> {
    Upward(&'static (dyn UpwardPass<T> + Send + Sync + 'static)),
    Downward(&'static (dyn DownwardPass<T> + Send + Sync + 'static)),
    Node(&'static (dyn NodePass<T> + Send + Sync + 'static)),
}

impl<T> AnyPass<T> {
    pub fn pass_id(&self) -> PassId {
        match self {
            Self::Upward(pass) => pass.pass_id(),
            Self::Downward(pass) => pass.pass_id(),
            Self::Node(pass) => pass.pass_id(),
        }
    }

    pub fn dependancies(&self) -> &'static [PassId] {
        match self {
            Self::Upward(pass) => pass.dependancies(),
            Self::Downward(pass) => pass.dependancies(),
            Self::Node(pass) => pass.dependancies(),
        }
    }

    fn resolve(
        &self,
        tree: &mut impl TreeView<T>,
        dirty_states: &mut DirtyNodeStates,
        nodes_updated: &FxDashSet<NodeId>,
        ctx: &SendAnyMap,
    ) {
        match self {
            Self::Downward(pass) => {
                resolve_downward_pass(tree, *pass, dirty_states, nodes_updated, ctx)
            }
            Self::Upward(pass) => {
                resolve_upward_pass(tree, *pass, dirty_states, nodes_updated, ctx)
            }
            Self::Node(pass) => resolve_node_pass(tree, *pass, dirty_states, nodes_updated, ctx),
        }
    }
}

pub fn resolve_passes<T, Tr: TreeView<T> + Sync + Send>(
    tree: &mut Tr,
    dirty_nodes: DirtyNodeStates,
    passes: Vec<&AnyPass<T>>,
    ctx: SendAnyMap,
) -> FxDashSet<NodeId> {
    resolve_passes_single_threaded(tree, dirty_nodes, passes, ctx)
    // TODO: multithreadeding has some safety issues currently that need to be resolved before it can be used
    // let dirty_states = Arc::new(dirty_nodes);
    // let mut resolved_passes: FxHashSet<PassId> = FxHashSet::default();
    // let mut resolving = Vec::new();
    // let nodes_updated = Arc::new(FxDashSet::default());
    // let ctx = Arc::new(ctx);
    // while !passes.is_empty() {
    //     let mut currently_borrowed = MemberMask::default();
    //     std::thread::scope(|s| {
    //         let mut i = 0;
    //         while i < passes.len() {
    //             let pass = &passes[i];
    //             let pass_id = pass.pass_id();
    //             let pass_mask = pass.mask();
    //             if pass
    //                 .dependancies()
    //                 .iter()
    //                 .all(|d| resolved_passes.contains(d) || *d == pass_id)
    //                 && !pass_mask.overlaps(currently_borrowed)
    //             {
    //                 let pass = passes.remove(i);
    //                 resolving.push(pass_id);
    //                 currently_borrowed |= pass_mask;
    //                 let dirty_states = dirty_states.clone();
    //                 let nodes_updated = nodes_updated.clone();
    //                 let ctx = ctx.clone();
    //                 let mut dirty = DirtyNodes::default();
    //                 // dirty_states.all_dirty(pass_id, &mut dirty, tree);
    //                 // this is safe because the member_mask acts as a per-member mutex and we have verified that the pass does not overlap with any other pass
    //                 let tree_mut_unbounded = unsafe { &mut *(tree as *mut Tr) };
    //                 s.spawn(move || {
    //                     pass.resolve(
    //                         tree_mut_unbounded,
    //                         dirty,
    //                         &dirty_states,
    //                         &nodes_updated,
    //                         &ctx,
    //                     );
    //                 });
    //             } else {
    //                 i += 1;
    //             }
    //         }
    //         // all passes are resolved at the end of the scope
    //     });
    //     resolved_passes.extend(resolving.iter().copied());
    //     resolving.clear()
    // }
    // std::sync::Arc::try_unwrap(nodes_updated).unwrap()
}

pub fn resolve_passes_single_threaded<T, Tr: TreeView<T>>(
    tree: &mut Tr,
    dirty_nodes: DirtyNodeStates,
    mut passes: Vec<&AnyPass<T>>,
    ctx: SendAnyMap,
) -> FxDashSet<NodeId> {
    let mut dirty_states = dirty_nodes;
    let mut resolved_passes: FxHashSet<PassId> = FxHashSet::default();
    let nodes_updated = Arc::new(FxDashSet::default());
    let ctx = Arc::new(ctx);
    while !passes.is_empty() {
        for (i, pass) in passes.iter().enumerate() {
            let pass_id = pass.pass_id();
            if pass
                .dependancies()
                .iter()
                .all(|d| resolved_passes.contains(d) || *d == pass_id)
            {
                let pass = passes.remove(i);
                let nodes_updated = nodes_updated.clone();
                let ctx = ctx.clone();
                pass.resolve(tree, &mut dirty_states, &nodes_updated, &ctx);
                resolved_passes.insert(pass_id);
                break;
            }
        }
    }
    std::sync::Arc::try_unwrap(nodes_updated).unwrap()
}

#[test]
fn node_pass() {
    use crate::tree::{Tree, TreeLike};
    let mut tree = Tree::new(0);

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
            MemberMask(0)
        }
    }

    impl NodePass<i32> for AddPass {
        fn pass(&self, node: &mut i32, _: &SendAnyMap) -> bool {
            *node += 1;
            true
        }
    }

    let add_pass = AnyPass::Node(&AddPass);
    let passes = vec![&add_pass];
    let mut dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
    dirty_nodes.insert(PassId(0), tree.root(), 0);
    resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

    assert_eq!(tree.get(tree.root()).unwrap(), &1);
}

#[test]
fn dependant_node_pass() {
    use crate::tree::{Tree, TreeLike};
    let mut tree = Tree::new(0);

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
            MemberMask(0)
        }
    }

    impl NodePass<i32> for AddPass {
        fn pass(&self, node: &mut i32, _: &SendAnyMap) -> bool {
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
            MemberMask(0)
        }
    }
    impl NodePass<i32> for SubtractPass {
        fn pass(&self, node: &mut i32, _: &SendAnyMap) -> bool {
            *node -= 1;
            true
        }
    }

    let add_pass = AnyPass::Node(&AddPass);
    let subtract_pass = AnyPass::Node(&SubtractPass);
    let passes = vec![&add_pass, &subtract_pass];
    let mut dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
    dirty_nodes.insert(PassId(1), tree.root(), 0);
    resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

    assert_eq!(*tree.get(tree.root()).unwrap(), 0);
}

#[test]
fn independant_node_pass() {
    use crate::tree::{Tree, TreeLike};
    let mut tree = Tree::new((0, 0));

    struct AddPass1;
    impl Pass for AddPass1 {
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
            MemberMask(0)
        }
    }

    impl NodePass<(i32, i32)> for AddPass1 {
        fn pass(&self, node: &mut (i32, i32), _: &SendAnyMap) -> bool {
            node.0 += 1;
            true
        }
    }

    struct AddPass2;
    impl Pass for AddPass2 {
        fn pass_id(&self) -> PassId {
            PassId(1)
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

    impl NodePass<(i32, i32)> for AddPass2 {
        fn pass(&self, node: &mut (i32, i32), _: &SendAnyMap) -> bool {
            node.1 += 1;
            true
        }
    }

    let add_pass1 = AnyPass::Node(&AddPass1);
    let add_pass2 = AnyPass::Node(&AddPass2);
    let passes = vec![&add_pass1, &add_pass2];
    let mut dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
    dirty_nodes.insert(PassId(0), tree.root(), 0);
    dirty_nodes.insert(PassId(1), tree.root(), 0);
    resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

    assert_eq!(tree.get(tree.root()).unwrap(), &(1, 1));
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
            MemberMask(0)
        }
    }
    impl DownwardPass<i32> for AddPass {
        fn pass(&self, node: &mut i32, parent: Option<&mut i32>, _: &SendAnyMap) -> PassReturn {
            if let Some(parent) = parent {
                *node += *parent;
            }
            PassReturn {
                progress: true,
                mark_dirty: true,
            }
        }
    }

    let add_pass = AnyPass::Downward(&AddPass);
    let passes = vec![&add_pass];
    let mut dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
    dirty_nodes.insert(PassId(0), tree.root(), 0);
    resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

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
            MemberMask(0)
        }
    }
    impl DownwardPass<i32> for AddPass {
        fn pass(&self, node: &mut i32, parent: Option<&mut i32>, _: &SendAnyMap) -> PassReturn {
            if let Some(parent) = parent {
                *node += *parent;
            } else {
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
            MemberMask(0)
        }
    }
    impl DownwardPass<i32> for SubtractPass {
        fn pass(&self, node: &mut i32, parent: Option<&mut i32>, _: &SendAnyMap) -> PassReturn {
            if let Some(parent) = parent {
                *node -= *parent;
            } else {
            }
            PassReturn {
                progress: true,
                mark_dirty: true,
            }
        }
    }

    let add_pass = AnyPass::Downward(&AddPass);
    let subtract_pass = AnyPass::Downward(&SubtractPass);
    let passes = vec![&add_pass, &subtract_pass];
    let mut dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
    dirty_nodes.insert(PassId(1), tree.root(), 0);
    resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

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
            MemberMask(0)
        }
    }
    impl UpwardPass<i32> for AddPass {
        fn pass<'a>(
            &self,
            node: &mut i32,
            children: &mut dyn Iterator<Item = &'a mut i32>,
            _: &SendAnyMap,
        ) -> PassReturn {
            *node += children.map(|i| *i).sum::<i32>();
            PassReturn {
                progress: true,
                mark_dirty: true,
            }
        }
    }

    let add_pass = AnyPass::Upward(&AddPass);
    let passes = vec![&add_pass];
    let mut dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
    dirty_nodes.insert(PassId(0), grandchild1, tree.height(grandchild1).unwrap());
    dirty_nodes.insert(PassId(0), grandchild2, tree.height(grandchild2).unwrap());
    resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

    assert_eq!(tree.get(tree.root()).unwrap(), &2);
    assert_eq!(tree.get(child1).unwrap(), &1);
    assert_eq!(tree.get(grandchild1).unwrap(), &1);
    assert_eq!(tree.get(child2).unwrap(), &1);
    assert_eq!(tree.get(grandchild2).unwrap(), &1);
}

#[test]
fn dependant_up_pass() {
    use crate::tree::{Tree, TreeLike};
    // 0
    let mut tree = Tree::new(0);
    let parent = tree.root();
    // 1
    let child1 = tree.create_node(0);
    tree.add_child(parent, child1);
    // 2
    let grandchild1 = tree.create_node(1);
    tree.add_child(child1, grandchild1);
    // 3
    let child2 = tree.create_node(0);
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
            MemberMask(0)
        }
    }
    impl UpwardPass<i32> for AddPass {
        fn pass<'a>(
            &self,
            node: &mut i32,
            children: &mut dyn Iterator<Item = &'a mut i32>,
            _: &SendAnyMap,
        ) -> PassReturn {
            *node += children.map(|i| *i).sum::<i32>();
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
            MemberMask(0)
        }
    }
    impl UpwardPass<i32> for SubtractPass {
        fn pass<'a>(
            &self,
            node: &mut i32,
            children: &mut dyn Iterator<Item = &'a mut i32>,
            _: &SendAnyMap,
        ) -> PassReturn {
            *node -= children.map(|i| *i).sum::<i32>();
            PassReturn {
                progress: true,
                mark_dirty: true,
            }
        }
    }

    let add_pass = AnyPass::Upward(&AddPass);
    let subtract_pass = AnyPass::Upward(&SubtractPass);
    let passes = vec![&add_pass, &subtract_pass];
    let mut dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
    dirty_nodes.insert(PassId(1), grandchild1, tree.height(grandchild1).unwrap());
    dirty_nodes.insert(PassId(1), grandchild2, tree.height(grandchild2).unwrap());
    resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

    // Tree before:
    // 0=\
    //   0=\
    //     1
    //   0=\
    //     1
    // Tree after subtract:
    // 2=\
    //   -1=\
    //      1
    //   -1=\
    //      1
    // Tree after add:
    // 2=\
    //   0=\
    //     1
    //   0=\
    //     1
    assert_eq!(tree.get(tree.root()).unwrap(), &2);
    assert_eq!(tree.get(child1).unwrap(), &0);
    assert_eq!(tree.get(grandchild1).unwrap(), &1);
    assert_eq!(tree.get(child2).unwrap(), &0);
    assert_eq!(tree.get(grandchild2).unwrap(), &1);
}
