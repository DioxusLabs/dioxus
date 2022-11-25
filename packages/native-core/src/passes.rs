use crossbeam_deque::{Injector, Stealer, Worker};
use parking_lot::RwLock;
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use std::hash::BuildHasherDefault;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::tree::{NodeId, SharedView, TreeView};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct PassId(u64);

pub trait UpwardPass<T> {
    fn pass_id(&self) -> PassId;
    fn dependancies(&self) -> &'static [PassId];
    fn dependants(&self) -> &'static [PassId];
    fn pass<'a>(&self, node: &mut T, children: &mut dyn Iterator<Item = &'a mut T>) -> bool;
}

pub trait DownwardPass<T> {
    fn pass_id(&self) -> PassId;
    fn dependancies(&self) -> &'static [PassId];
    fn dependants(&self) -> &'static [PassId];
    fn pass(&self, node: &mut T, parent: Option<&mut T>) -> bool;
}

pub trait NodePass<T> {
    fn pass_id(&self) -> PassId;
    fn dependancies(&self) -> &'static [PassId];
    fn dependants(&self) -> &'static [PassId];
    fn pass(&self, node: &mut T) -> bool;
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
}

type FxDashMap<K, V> = dashmap::DashMap<K, V, BuildHasherDefault<FxHasher>>;

#[derive(Default)]
struct DirtyNodeStates {
    dirty: FxDashMap<NodeId, Vec<AtomicU64>>,
}

impl DirtyNodeStates {
    fn new(starting_nodes: FxHashMap<NodeId, FxHashSet<PassId>>) -> Self {
        let this = Self::default();
        for (node, nodes) in starting_nodes {
            for pass_id in nodes {
                this.insert(pass_id, node);
            }
        }
        this
    }

    fn insert(&self, pass_id: PassId, node_id: NodeId) {
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

    fn all_dirty(&self, pass_id: PassId) -> impl Iterator<Item = NodeId> + '_ {
        let pass_id = pass_id.0;
        let index = pass_id / 64;
        let bit = pass_id % 64;
        let encoded = 1 << bit;
        self.dirty.iter().filter_map(move |entry| {
            let node_id = entry.key();
            let dirty = entry.value();
            if let Some(atomic) = dirty.get(index as usize) {
                if atomic.load(Ordering::Relaxed) & encoded != 0 {
                    Some(*node_id)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}

fn get_pass<T, Tr: TreeView<T>>(
    passes: &mut Vec<AnyPass<T>>,
    resolved_passes: &mut FxHashSet<PassId>,
    dirty_nodes: &DirtyNodeStates,
    shared_view: &mut SharedView<T, Tr>,
    global: &Injector<NodeId>,
    current_pass: &RwLock<Option<AnyPass<T>>>,
) {
    for i in 0..passes.len() {
        if passes[i]
            .dependancies()
            .iter()
            .all(|id| resolved_passes.contains(id))
        {
            let pass = passes.remove(i);
            let pass_id = pass.pass_id();
            resolved_passes.insert(pass_id);
            match pass {
                AnyPass::Upward(pass) => {
                    // Upward passes are more difficult. Right now we limit them to only one thread.
                    let worker = Worker::new_fifo();
                    let mut queued_nodes = FxHashSet::default();
                    for node in dirty_nodes.all_dirty(pass_id) {
                        queued_nodes.insert(node);
                        worker.push(node);
                    }
                    while let Some(id) = worker.pop() {
                        let (node, mut children) = shared_view.parent_child_mut(id).unwrap();
                        if pass.pass(node, &mut children) {
                            drop(children);
                            if let Some(id) = shared_view.parent_id(id) {
                                for dependant in pass.dependants() {
                                    dirty_nodes.insert(*dependant, id);
                                }
                                if !queued_nodes.contains(&id) {
                                    queued_nodes.insert(id);
                                    worker.push(id);
                                }
                            }
                        }
                    }
                }
                _ => {
                    for node in dirty_nodes.all_dirty(pass_id) {
                        global.push(node);
                    }
                    current_pass.write().replace(pass);
                }
            }

            break;
        }
    }
}

pub fn resolve_passes<T>(
    tree: &mut impl TreeView<T>,
    starting_nodes: FxHashMap<NodeId, FxHashSet<PassId>>,
    mut passes: Vec<AnyPass<T>>,
) {
    let dirty_nodes: Arc<DirtyNodeStates> = Arc::new(DirtyNodeStates::new(starting_nodes));
    let global = Injector::default();

    let core_count = thread::available_parallelism()
        .map(|c| c.get())
        .unwrap_or(1);
    let workers: Vec<Worker<NodeId>> = (0..core_count).map(|_| Worker::new_fifo()).collect();
    let stealers: Vec<_> = workers.iter().map(|w| w.stealer()).collect();
    let mut shared_view = SharedView::new(tree);
    let mut resolved_passes: FxHashSet<PassId> = FxHashSet::default();
    let current_pass: Arc<RwLock<Option<AnyPass<T>>>> = Arc::new(RwLock::new(None));

    thread::scope(|s| {
        get_pass(
            &mut passes,
            &mut resolved_passes,
            &dirty_nodes,
            &mut shared_view,
            &global,
            &current_pass,
        );
        let global = &global;
        let stealers = &stealers;
        for (_, w) in (0..core_count).zip(workers.into_iter()) {
            let mut shared_view = shared_view.clone();
            let current_pass = current_pass.clone();
            let dirty_nodes = dirty_nodes.clone();
            s.spawn(move || {
                while let Some(current_pass) = &*current_pass.read() {
                    match current_pass {
                        AnyPass::Upward(_) => {
                            todo!("Upward passes are single threaded")
                        }
                        AnyPass::Node(pass) => {
                            // Node passes are the easiest to parallelize. We just run the pass on each node.
                            while let Some(id) = find_task(&w, global, stealers) {
                                let node = shared_view.get_mut(id).unwrap();
                                if pass.pass(node) {
                                    for dependant in pass.dependants() {
                                        dirty_nodes.insert(*dependant, id);
                                    }
                                }
                            }
                        }
                        AnyPass::Downward(pass) => {
                            // Downward passes are easy to parallelize. We try to keep trees localized to one thread, but allow work stealing to balance the load.
                            while let Some(id) = find_task(&w, global, stealers) {
                                let (node, parent) = shared_view.node_parent_mut(id).unwrap();
                                if pass.pass(node, parent) {
                                    for id in shared_view.children_ids(id).unwrap() {
                                        for dependant in pass.dependants() {
                                            dirty_nodes.insert(*dependant, *id);
                                        }
                                        w.push(*id);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
        while !passes.is_empty() {
            while !stealers.iter().all(|s| s.is_empty()) {
                std::thread::sleep(Duration::from_millis(50));
            }
            get_pass(
                &mut passes,
                &mut resolved_passes,
                &dirty_nodes,
                &mut shared_view,
                global,
                &current_pass,
            );
        }
        *current_pass.write() = None;
    });
}

fn find_task<T>(local: &Worker<T>, global: &Injector<T>, stealers: &[Stealer<T>]) -> Option<T> {
    // Pop a task from the local queue, if not empty.
    local.pop().or_else(|| {
        // Otherwise, we need to look for a task elsewhere.
        std::iter::repeat_with(|| {
            // Try stealing a batch of tasks from the global queue.
            global
                .steal_batch_and_pop(local)
                // Or try stealing a task from one of the other threads.
                .or_else(|| stealers.iter().map(|s| s.steal()).collect())
        })
        // Loop while no task was stolen and any steal operation needs to be retried.
        .find(|s| !s.is_retry())
        // Extract the stolen task, if there is one.
        .and_then(|s| s.success())
    })
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

    struct AddPass;

    impl NodePass<i32> for AddPass {
        fn pass_id(&self) -> PassId {
            PassId(0)
        }

        fn dependancies(&self) -> &'static [PassId] {
            &[]
        }

        fn dependants(&self) -> &'static [PassId] {
            &[]
        }

        fn pass(&self, node: &mut i32) -> bool {
            *node += 1;
            true
        }
    }

    let passes = vec![AnyPass::Node(Box::new(AddPass))];
    let mut dirty_nodes: FxHashMap<NodeId, FxHashSet<PassId>> = FxHashMap::default();
    dirty_nodes.insert(tree.root(), [PassId(0)].into_iter().collect());
    resolve_passes(&mut tree, dirty_nodes, passes);

    assert_eq!(tree.get(tree.root()).unwrap(), &1);
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

    impl DownwardPass<i32> for AddPass {
        fn pass_id(&self) -> PassId {
            PassId(0)
        }

        fn dependancies(&self) -> &'static [PassId] {
            &[]
        }

        fn dependants(&self) -> &'static [PassId] {
            &[]
        }

        fn pass(&self, node: &mut i32, parent: Option<&mut i32>) -> bool {
            if let Some(parent) = parent {
                *node += *parent;
            }
            true
        }
    }

    let passes = vec![AnyPass::Downward(Box::new(AddPass))];
    let mut dirty_nodes: FxHashMap<NodeId, FxHashSet<PassId>> = FxHashMap::default();
    dirty_nodes.insert(tree.root(), [PassId(0)].into_iter().collect());
    resolve_passes(&mut tree, dirty_nodes, passes);

    assert_eq!(tree.get(tree.root()).unwrap(), &1);
    assert_eq!(tree.get(child1).unwrap(), &2);
    assert_eq!(tree.get(grandchild1).unwrap(), &3);
    assert_eq!(tree.get(child2).unwrap(), &2);
    assert_eq!(tree.get(grandchild2).unwrap(), &3);
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

    impl UpwardPass<i32> for AddPass {
        fn pass_id(&self) -> PassId {
            PassId(0)
        }

        fn dependancies(&self) -> &'static [PassId] {
            &[]
        }

        fn dependants(&self) -> &'static [PassId] {
            &[]
        }

        fn pass<'a>(
            &self,
            node: &mut i32,
            children: &mut dyn Iterator<Item = &'a mut i32>,
        ) -> bool {
            *node += children.map(|i| *i).sum::<i32>();
            true
        }
    }

    let passes = vec![AnyPass::Upward(Box::new(AddPass))];
    let mut dirty_nodes: FxHashMap<NodeId, FxHashSet<PassId>> = FxHashMap::default();
    dirty_nodes.insert(grandchild1, [PassId(0)].into_iter().collect());
    dirty_nodes.insert(grandchild2, [PassId(0)].into_iter().collect());
    resolve_passes(&mut tree, dirty_nodes, passes);

    assert_eq!(tree.get(tree.root()).unwrap(), &2);
    assert_eq!(tree.get(child1).unwrap(), &1);
    assert_eq!(tree.get(grandchild1).unwrap(), &1);
    assert_eq!(tree.get(child2).unwrap(), &1);
    assert_eq!(tree.get(grandchild2).unwrap(), &1);
}
