use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum LifecycleRole {
    ComponentA,
    ComponentB,
    SuspenseBoundary,
    SuspenseChild,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LifecycleKey {
    pub(crate) role: LifecycleRole,
    pub(crate) id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum LifecycleRun {
    Incremental,
    Fresh,
}

pub(crate) type LifecycleSnapshot = BTreeMap<LifecycleKey, usize>;

thread_local! {
    static CURRENT_RUN: Cell<Option<LifecycleRun>> = const { Cell::new(None) };
    static LIVE_COMPONENTS: RefCell<BTreeMap<(LifecycleRun, LifecycleKey, LifecycleContext), usize>> = RefCell::new(BTreeMap::new());
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct LifecycleContext {
    suspense_ancestors: Vec<u64>,
}

impl LifecycleContext {
    fn new(suspense_ancestors: &[u64]) -> Self {
        Self {
            suspense_ancestors: suspense_ancestors.to_vec(),
        }
    }

    fn intersects_suspense_ids(&self, suspense_ids: &BTreeSet<u64>) -> bool {
        self.suspense_ancestors
            .iter()
            .any(|id| suspense_ids.contains(id))
    }
}

pub(crate) fn reset_all() {
    CURRENT_RUN.with(|run| run.set(None));
    LIVE_COMPONENTS.with(|live| live.borrow_mut().clear());
}

pub(crate) fn reset_run(run: LifecycleRun) {
    LIVE_COMPONENTS.with(|live| {
        live.borrow_mut()
            .retain(|(live_run, _, _), _| *live_run != run);
    });
}

pub(crate) fn with_run<R>(run: LifecycleRun, f: impl FnOnce() -> R) -> R {
    struct RunGuard(Option<LifecycleRun>);

    impl Drop for RunGuard {
        fn drop(&mut self) {
            CURRENT_RUN.with(|run| run.set(self.0));
        }
    }

    let previous = CURRENT_RUN.with(|current| current.replace(Some(run)));
    let _guard = RunGuard(previous);
    f()
}

pub(crate) fn track(
    role: LifecycleRole,
    id: u64,
    suspense_ancestors: &[u64],
) -> Rc<LifecycleGuard> {
    let run = CURRENT_RUN.with(Cell::get);
    let key = LifecycleKey { role, id };
    let context = LifecycleContext::new(suspense_ancestors);
    increment(run, key, &context);
    Rc::new(LifecycleGuard {
        run: Cell::new(run),
        key: Cell::new(key),
        context: RefCell::new(context),
    })
}

pub(crate) fn snapshot(run: LifecycleRun) -> LifecycleSnapshot {
    LIVE_COMPONENTS.with(|live| {
        let mut out = LifecycleSnapshot::new();
        for ((live_run, key, _), count) in live.borrow().iter() {
            if *live_run == run {
                *out.entry(*key).or_insert(0) += *count;
            }
        }
        out
    })
}

pub(crate) fn snapshot_with_suspense_ancestor(
    run: LifecycleRun,
    suspense_ids: &BTreeSet<u64>,
) -> LifecycleSnapshot {
    LIVE_COMPONENTS.with(|live| {
        let mut out = LifecycleSnapshot::new();
        for ((live_run, key, context), count) in live.borrow().iter() {
            if *live_run == run && context.intersects_suspense_ids(suspense_ids) {
                *out.entry(*key).or_insert(0) += *count;
            }
        }
        out
    })
}

#[derive(Debug)]
pub(crate) struct LifecycleGuard {
    run: Cell<Option<LifecycleRun>>,
    key: Cell<LifecycleKey>,
    context: RefCell<LifecycleContext>,
}

impl LifecycleGuard {
    pub(crate) fn update(&self, role: LifecycleRole, id: u64, suspense_ancestors: &[u64]) {
        let next_run = CURRENT_RUN.with(Cell::get);
        let next_key = LifecycleKey { role, id };
        let next_context = LifecycleContext::new(suspense_ancestors);
        let current_run = self.run.get();
        let current_key = self.key.get();
        let current_context = self.context.borrow().clone();

        if current_run == next_run && current_key == next_key && current_context == next_context {
            return;
        }

        decrement(current_run, current_key, &current_context);
        increment(next_run, next_key, &next_context);
        self.run.set(next_run);
        self.key.set(next_key);
        self.context.replace(next_context);
    }
}

impl Drop for LifecycleGuard {
    fn drop(&mut self) {
        let context = self.context.get_mut();
        decrement(self.run.get(), self.key.get(), context);
    }
}

fn increment(run: Option<LifecycleRun>, key: LifecycleKey, context: &LifecycleContext) {
    if let Some(run) = run {
        LIVE_COMPONENTS.with(|live| {
            *live
                .borrow_mut()
                .entry((run, key, context.clone()))
                .or_insert(0) += 1;
        });
    }
}

fn decrement(run: Option<LifecycleRun>, key: LifecycleKey, context: &LifecycleContext) {
    let Some(run) = run else {
        return;
    };
    LIVE_COMPONENTS.with(|live| {
        let mut live = live.borrow_mut();
        let live_key = (run, key, context.clone());
        let Some(count) = live.get_mut(&live_key) else {
            return;
        };
        if *count <= 1 {
            live.remove(&live_key);
        } else {
            *count -= 1;
        }
    });
}
