use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::{Rc, Weak},
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

#[derive(Clone, Default)]
pub(crate) struct LifecycleState {
    inner: Rc<LifecycleStateInner>,
}

#[derive(Default)]
struct LifecycleStateInner {
    current_run: Cell<Option<LifecycleRun>>,
    live_components: RefCell<BTreeMap<(LifecycleRun, LifecycleKey, LifecycleContext), usize>>,
    live_guards: RefCell<Vec<Weak<LifecycleGuard>>>,
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

    fn retargeted_suspense_ancestor(
        &self,
        old_parent: &Self,
        old_id: u64,
        new_parent: &Self,
        new_id: u64,
    ) -> Option<Self> {
        let old_prefix = &old_parent.suspense_ancestors;
        if !self.suspense_ancestors.starts_with(old_prefix) {
            return None;
        }

        let after_parent = &self.suspense_ancestors[old_prefix.len()..];
        let [first, suffix @ ..] = after_parent else {
            return None;
        };
        if *first != old_id {
            return None;
        }

        let mut suspense_ancestors = new_parent.suspense_ancestors.clone();
        suspense_ancestors.push(new_id);
        suspense_ancestors.extend_from_slice(suffix);
        Some(Self { suspense_ancestors })
    }
}

impl LifecycleState {
    pub(crate) fn reset_all(&self) {
        self.inner.current_run.set(None);
        self.inner.live_components.borrow_mut().clear();
        self.inner.live_guards.borrow_mut().clear();
    }

    pub(crate) fn reset_run(&self, run: LifecycleRun) {
        self.inner
            .live_components
            .borrow_mut()
            .retain(|(live_run, _, _), _| *live_run != run);
    }

    pub(crate) fn with_run<R>(&self, run: LifecycleRun, f: impl FnOnce() -> R) -> R {
        struct RunGuard {
            state: LifecycleState,
            previous: Option<LifecycleRun>,
        }

        impl Drop for RunGuard {
            fn drop(&mut self) {
                self.state.inner.current_run.set(self.previous);
            }
        }

        let previous = self.inner.current_run.replace(Some(run));
        let _guard = RunGuard {
            state: self.clone(),
            previous,
        };
        f()
    }

    pub(crate) fn track(
        &self,
        role: LifecycleRole,
        id: u64,
        suspense_ancestors: &[u64],
    ) -> Rc<LifecycleGuard> {
        let run = self.inner.current_run.get();
        let key = LifecycleKey { role, id };
        let context = LifecycleContext::new(suspense_ancestors);
        self.increment(run, key, &context);
        let guard = Rc::new(LifecycleGuard {
            state: self.clone(),
            run: Cell::new(run),
            key: Cell::new(key),
            context: RefCell::new(context),
        });
        self.inner
            .live_guards
            .borrow_mut()
            .push(Rc::downgrade(&guard));
        guard
    }

    pub(crate) fn snapshot(&self, run: LifecycleRun) -> LifecycleSnapshot {
        let mut out = LifecycleSnapshot::new();
        for ((live_run, key, _), count) in self.inner.live_components.borrow().iter() {
            if *live_run == run {
                *out.entry(*key).or_insert(0) += *count;
            }
        }
        out
    }

    pub(crate) fn snapshot_with_suspense_ancestor(
        &self,
        run: LifecycleRun,
        suspense_ids: &BTreeSet<u64>,
    ) -> LifecycleSnapshot {
        let mut out = LifecycleSnapshot::new();
        for ((live_run, key, context), count) in self.inner.live_components.borrow().iter() {
            if *live_run == run && context.intersects_suspense_ids(suspense_ids) {
                *out.entry(*key).or_insert(0) += *count;
            }
        }
        out
    }

    pub(crate) fn debug_snapshot(&self, run: LifecycleRun) -> String {
        let mut out = String::new();
        for ((live_run, key, context), count) in self.inner.live_components.borrow().iter() {
            if *live_run == run {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&format!(
                    "{key:?} x{count} in {:?}",
                    context.suspense_ancestors
                ));
            }
        }
        out
    }

    fn increment(&self, run: Option<LifecycleRun>, key: LifecycleKey, context: &LifecycleContext) {
        if let Some(run) = run {
            *self
                .inner
                .live_components
                .borrow_mut()
                .entry((run, key, context.clone()))
                .or_insert(0) += 1;
        }
    }

    fn decrement(&self, run: Option<LifecycleRun>, key: LifecycleKey, context: &LifecycleContext) {
        let Some(run) = run else {
            return;
        };
        let mut live = self.inner.live_components.borrow_mut();
        let live_key = (run, key, context.clone());
        let Some(count) = live.get_mut(&live_key) else {
            return;
        };
        if *count <= 1 {
            live.remove(&live_key);
        } else {
            *count -= 1;
        }
    }

    fn retarget_suspense_descendant_contexts(
        &self,
        run: Option<LifecycleRun>,
        old_id: u64,
        new_id: u64,
        old_parent: &LifecycleContext,
        new_parent: &LifecycleContext,
    ) {
        let Some(run) = run else {
            return;
        };

        let retargeted = {
            let mut retargeted = Vec::new();
            self.inner.live_guards.borrow_mut().retain(|guard| {
                let Some(guard) = guard.upgrade() else {
                    return false;
                };

                if guard.run.get() == Some(run) {
                    let current_context = guard.context.borrow().clone();
                    if let Some(next_context) = current_context
                        .retargeted_suspense_ancestor(old_parent, old_id, new_parent, new_id)
                    {
                        if next_context != current_context {
                            let key = guard.key.get();
                            guard.context.replace(next_context.clone());
                            retargeted.push((key, current_context, next_context));
                        }
                    }
                }

                true
            });
            retargeted
        };

        for (key, current_context, next_context) in retargeted {
            self.decrement(Some(run), key, &current_context);
            self.increment(Some(run), key, &next_context);
        }
    }
}

pub(crate) struct LifecycleGuard {
    state: LifecycleState,
    run: Cell<Option<LifecycleRun>>,
    key: Cell<LifecycleKey>,
    context: RefCell<LifecycleContext>,
}

impl LifecycleGuard {
    pub(crate) fn update(&self, role: LifecycleRole, id: u64, suspense_ancestors: &[u64]) {
        let next_run = self.state.inner.current_run.get();
        let next_key = LifecycleKey { role, id };
        let next_context = LifecycleContext::new(suspense_ancestors);
        let current_run = self.run.get();
        let current_key = self.key.get();
        let current_context = self.context.borrow().clone();

        if current_run == next_run && current_key == next_key && current_context == next_context {
            return;
        }

        if current_key.role == LifecycleRole::SuspenseBoundary
            && next_key.role == LifecycleRole::SuspenseBoundary
            && current_key.id != next_key.id
        {
            // A reused suspense boundary can keep descendants alive without
            // rerendering them, so retarget their recorded ancestry to the
            // boundary identity observed by the current render.
            self.state.retarget_suspense_descendant_contexts(
                current_run,
                current_key.id,
                next_key.id,
                &current_context,
                &next_context,
            );
        }

        self.state
            .decrement(current_run, current_key, &current_context);
        self.state.increment(next_run, next_key, &next_context);
        self.run.set(next_run);
        self.key.set(next_key);
        self.context.replace(next_context);
    }
}

impl Drop for LifecycleGuard {
    fn drop(&mut self) {
        let context = self.context.get_mut();
        self.state
            .decrement(self.run.get(), self.key.get(), context);
    }
}
