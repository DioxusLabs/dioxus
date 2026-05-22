use super::{FiberCommit, FiberStep, RenderStats, UpdatePriority};
use crate::mutations::{DiffDispatch, RenderTargetWriter};
use crate::runtime::RuntimeGuard;
use crate::{RenderTargetId, VirtualDom};
use std::collections::BTreeMap;

/// Low-level cooperative render driver.
///
/// The driver advances virtual-DOM work one unit at a time. Each work unit
/// writes directly into the renderer-supplied writers held in
/// `VirtualDom::targets` — the dispatcher routes calls by the runtime's active
/// `RenderTargetId`. Call [`Self::commit`] to mark accumulated writes durable
/// (each registered writer's `commit` runs); commit can be skipped and
/// [`Self::discard`] called instead to roll a fiber's work back.
pub(crate) struct FiberDriver<'a> {
    dom: &'a mut VirtualDom,
    /// Owned for the lifetime of this driver; restored to `dom.targets` on
    /// drop. Moved out at construction so the diff hot path can mutably borrow
    /// the registry without colliding with `&mut self.dom`.
    targets: BTreeMap<RenderTargetId, Box<dyn RenderTargetWriter>>,
    /// Mutations recorded since the last commit. Used as the `mutation_count`
    /// field on `FiberCommit` and to gate `pending_commit`.
    pending_mutations: usize,
    job: RenderJob,
    must_commit: Option<FiberCommit>,
    finished: bool,
}

struct RenderJob {
    stats: RenderStats,
    commit_priority: UpdatePriority,
    work_since_commit: bool,
    work_count_since_commit: usize,
    work_done: usize,
}

impl RenderJob {
    fn new() -> Self {
        Self {
            stats: RenderStats::default(),
            commit_priority: UpdatePriority::Idle,
            work_since_commit: false,
            work_count_since_commit: 0,
            work_done: 0,
        }
    }

    fn record_work(&mut self, priority: UpdatePriority, requires_commit: bool) {
        self.stats.priority = self.stats.priority.min(priority);
        if requires_commit {
            self.commit_priority = self.commit_priority.min(priority);
            self.work_since_commit = true;
            self.work_count_since_commit += 1;
        }
        self.work_done += 1;
        self.stats.work_count += 1;
    }

    fn record_yield(&mut self) {
        self.stats.yield_count += 1;
    }

    fn reset_yield_budget(&mut self) {
        self.work_done = 0;
    }

    fn pending_commit(&self, dom: &VirtualDom, mutation_count: usize) -> Option<FiberCommit> {
        self.work_since_commit.then_some(FiberCommit {
            priority: self.commit_priority,
            work_count: self.work_count_since_commit,
            mutation_count,
            generation: dom.commit_generation + 1,
        })
    }

    fn mark_committed(
        &mut self,
        dom: &mut VirtualDom,
        pending_mutations: usize,
    ) -> Option<FiberCommit> {
        let mut commit = self.pending_commit(dom, pending_mutations)?;
        dom.commit_generation += 1;
        self.stats.generation = dom.commit_generation;
        self.stats.commit_count += 1;
        commit.generation = dom.commit_generation;
        self.commit_priority = UpdatePriority::Idle;
        self.work_since_commit = false;
        self.work_count_since_commit = 0;
        Some(commit)
    }
}

impl<'a> FiberDriver<'a> {
    pub(crate) fn next_fiber(&mut self) -> FiberStep {
        if self.finished {
            return FiberStep::Idle(self.job.stats);
        }

        if let Some(commit) = self.must_commit {
            self.must_commit = Some(commit);
            return FiberStep::MustCommit;
        }

        let _runtime = RuntimeGuard::new(self.dom.runtime.clone());
        self.dom.queue_events();
        if self.job.work_since_commit
            && self
                .dom
                .next_work_priority()
                .is_some_and(|priority| priority != self.job.commit_priority)
            && let Some(commit) = self.pending_commit()
        {
            self.must_commit = Some(commit);
            return FiberStep::MustCommit;
        }

        loop {
            let Some(work) = self.dom.pop_work() else {
                if let Some(commit) = self.pending_commit() {
                    self.must_commit = Some(commit);
                    return FiberStep::MustCommit;
                }

                self.dom.runtime.finish_render();
                self.finished = true;
                return FiberStep::Idle(self.job.stats);
            };

            let requires_commit = work.requires_commit();
            let runtime = self.dom.runtime.clone();
            let mut dispatch = DiffDispatch::new(&mut self.targets, runtime);
            dispatch.auto_create_targets = self.dom.auto_create_targets;
            let priority = self.dom.render_work_into(&mut dispatch, work);
            self.pending_mutations += dispatch.mutation_count;
            self.job.record_work(priority, requires_commit);

            self.dom.queue_events();
            let must_commit_before_next = self
                .dom
                .next_work_priority()
                .is_some_and(|next_priority| next_priority != self.job.commit_priority);

            if must_commit_before_next {
                self.must_commit = self.pending_commit();
            }

            return FiberStep::Ran;
        }
    }

    pub(crate) fn commit(&mut self) -> Option<FiberCommit> {
        let pending = self.pending_mutations;
        let commit = self.job.mark_committed(self.dom, pending)?;
        // Commit each target's writer, then drain that target's effects so
        // effects observe DOM that already reflects this target's edits.
        let runtime = self.dom.runtime.clone();
        for (target_id, writer) in self.targets.iter_mut() {
            writer.commit();
            for effect in runtime.drain_effects_for_target(*target_id) {
                effect.run();
            }
        }
        self.pending_mutations = 0;
        self.must_commit = None;
        Some(commit)
    }

    pub(crate) fn yield_now(&mut self) {
        self.job.record_yield();
        self.job.reset_yield_budget();
        self.dom.queue_events();
    }

    fn pending_commit(&self) -> Option<FiberCommit> {
        self.job.pending_commit(self.dom, self.pending_mutations)
    }
}

impl<'a> Drop for FiberDriver<'a> {
    fn drop(&mut self) {
        // Restore the registry to the VDom so subsequent renders see the
        // renderer's writers.
        self.dom.targets = std::mem::take(&mut self.targets);
        if !self.finished {
            self.dom.runtime.finish_render();
        }
    }
}

impl VirtualDom {
    pub(crate) fn fiber_driver(&mut self) -> FiberDriver<'_> {
        self.queue_events();
        let targets = std::mem::take(&mut self.targets);
        FiberDriver {
            dom: self,
            targets,
            pending_mutations: 0,
            job: RenderJob::new(),
            must_commit: None,
            finished: false,
        }
    }
}
