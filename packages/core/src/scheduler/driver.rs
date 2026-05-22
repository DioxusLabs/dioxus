use super::{
    FiberCheckpoint, FiberCommit, FiberInfo, FiberPhase, FiberStep, RenderStats, UpdatePriority,
    Work,
};
use crate::arena::ElementId;
use crate::fiber::FiberId;
use crate::runtime::{Runtime, RuntimeGuard};
use crate::scopes::ScopeId;
use crate::{AttributeValue, RenderTargetId, Template, VirtualDom, WriteMutations};
use std::rc::Rc;

/// Low-level cooperative render driver.
///
/// The driver advances virtual-DOM work one unit at a time into an internal
/// mutation buffer. Call [`Self::commit`] to replay buffered mutations into a
/// renderer and make the completed work visible.
pub(crate) struct FiberDriver<'a> {
    dom: &'a mut VirtualDom,
    buffer: BufferedMutations,
    job: RenderJob,
    must_commit: Option<FiberCommit>,
    finished: bool,
}

#[derive(Debug)]
struct BufferedMutationRecord {
    target_id: RenderTargetId,
    op: BufferedMutation,
}

#[derive(Debug)]
enum BufferedMutation {
    AppendChildren {
        id: ElementId,
        m: usize,
    },
    AssignId {
        path: &'static [u8],
        id: ElementId,
    },
    CreateTextNode {
        value: String,
        id: ElementId,
    },
    LoadTemplate {
        template: Template,
        index: usize,
        id: ElementId,
    },
    ReplaceWith {
        id: ElementId,
        m: usize,
    },
    InsertChildrenAtPath {
        path: &'static [u8],
        m: usize,
    },
    InsertAfter {
        id: ElementId,
        m: usize,
    },
    InsertBefore {
        id: ElementId,
        m: usize,
    },
    SetAttribute {
        name: &'static str,
        ns: Option<&'static str>,
        value: AttributeValue,
        id: ElementId,
    },
    SetText {
        value: String,
        id: ElementId,
    },
    NewEventListener {
        name: &'static str,
        id: ElementId,
    },
    RemoveEventListener {
        name: &'static str,
        id: ElementId,
    },
    Remove {
        id: ElementId,
    },
    PushRoot {
        id: ElementId,
    },
    PopRoot,
}

impl BufferedMutation {
    fn replay(self, to: &mut impl WriteMutations) {
        match self {
            Self::AppendChildren { id, m } => to.append_children(id, m),
            Self::AssignId { path, id } => to.assign_node_id(path, id),
            Self::CreateTextNode { value, id } => to.create_text_node(&value, id),
            Self::LoadTemplate {
                template,
                index,
                id,
            } => to.load_template(template, index, id),
            Self::ReplaceWith { id, m } => to.replace_node_with(id, m),
            Self::InsertChildrenAtPath { path, m } => to.insert_children_at_path(path, m),
            Self::InsertAfter { id, m } => to.insert_nodes_after(id, m),
            Self::InsertBefore { id, m } => to.insert_nodes_before(id, m),
            Self::SetAttribute {
                name,
                ns,
                value,
                id,
            } => to.set_attribute(name, ns, &value, id),
            Self::SetText { value, id } => to.set_node_text(&value, id),
            Self::NewEventListener { name, id } => to.create_event_listener(name, id),
            Self::RemoveEventListener { name, id } => to.remove_event_listener(name, id),
            Self::Remove { id } => to.remove_node(id),
            Self::PushRoot { id } => to.push_root(id),
            Self::PopRoot => to.pop_root(),
        }
    }
}

struct BufferedMutations {
    runtime: Rc<Runtime>,
    edits: Vec<BufferedMutationRecord>,
}

impl BufferedMutations {
    fn new(runtime: Rc<Runtime>) -> Self {
        Self {
            runtime,
            edits: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.edits.len()
    }

    fn push(&mut self, op: BufferedMutation) {
        self.edits.push(BufferedMutationRecord {
            target_id: self.runtime.current_render_target_id(),
            op,
        });
    }

    fn drain_into(&mut self, to: &mut impl WriteMutations) {
        for BufferedMutationRecord { target_id, op } in self.edits.drain(..) {
            self.runtime.with_render_target(target_id, || op.replay(to));
        }
    }
}

impl WriteMutations for BufferedMutations {
    fn append_children(&mut self, id: ElementId, m: usize) {
        self.push(BufferedMutation::AppendChildren { id, m });
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        self.push(BufferedMutation::AssignId { path, id });
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        self.push(BufferedMutation::CreateTextNode {
            value: value.to_string(),
            id,
        });
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        self.push(BufferedMutation::LoadTemplate {
            template,
            index,
            id,
        });
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        self.push(BufferedMutation::ReplaceWith { id, m });
    }

    fn insert_children_at_path(&mut self, path: &'static [u8], m: usize) {
        self.push(BufferedMutation::InsertChildrenAtPath { path, m });
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        self.push(BufferedMutation::InsertAfter { id, m });
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        self.push(BufferedMutation::InsertBefore { id, m });
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        self.push(BufferedMutation::SetAttribute {
            name,
            ns,
            value: value.clone(),
            id,
        });
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        self.push(BufferedMutation::SetText {
            value: value.to_string(),
            id,
        });
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        self.push(BufferedMutation::NewEventListener { name, id });
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        self.push(BufferedMutation::RemoveEventListener { name, id });
    }

    fn remove_node(&mut self, id: ElementId) {
        self.push(BufferedMutation::Remove { id });
    }

    fn push_root(&mut self, id: ElementId) {
        self.push(BufferedMutation::PushRoot { id });
    }

    fn pop_root(&mut self) {
        self.push(BufferedMutation::PopRoot);
    }
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

    fn commit_buffered<M: WriteMutations>(
        &mut self,
        dom: &mut VirtualDom,
        buffer: &mut BufferedMutations,
        to: &mut M,
    ) -> Option<FiberCommit> {
        let mut commit = self.pending_commit(dom, buffer.len())?;
        buffer.drain_into(to);
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
            let info = self.dom.fiber_info_for_work(&work);
            let priority = self.dom.render_work_into(&mut self.buffer, work);
            self.job.record_work(priority, requires_commit);

            self.dom.queue_events();
            let next_priority = self.dom.next_work_priority();
            let has_higher_priority_work =
                next_priority.is_some_and(|next_priority| next_priority < priority);
            let must_commit_before_next = next_priority
                .is_some_and(|next_priority| next_priority != self.job.commit_priority);
            let checkpoint = FiberCheckpoint {
                work: info,
                work_count: self.job.work_done,
                pending_mutations: self.buffer.len(),
                has_higher_priority_work,
                must_commit_before_next,
            };

            if must_commit_before_next {
                self.must_commit = self.pending_commit();
            }

            return FiberStep::Ran(checkpoint);
        }
    }

    pub(crate) fn commit(&mut self, to: &mut impl WriteMutations) -> Option<FiberCommit> {
        let commit = self.job.commit_buffered(self.dom, &mut self.buffer, to)?;
        self.must_commit = None;
        Some(commit)
    }

    pub(crate) fn yield_now(&mut self) {
        self.job.record_yield();
        self.job.reset_yield_budget();
        self.dom.queue_events();
    }

    fn pending_commit(&self) -> Option<FiberCommit> {
        self.job.pending_commit(self.dom, self.buffer.len())
    }
}

impl VirtualDom {
    pub(crate) fn fiber_driver(&mut self) -> FiberDriver<'_> {
        let runtime = self.runtime.clone();
        self.queue_events();
        FiberDriver {
            dom: self,
            buffer: BufferedMutations::new(runtime),
            job: RenderJob::new(),
            must_commit: None,
            finished: false,
        }
    }

    fn fiber_info_for_work(&self, work: &Work) -> FiberInfo {
        match work {
            Work::DiffFiber(fiber) => FiberInfo {
                id: self.fiber_id_for_scope(fiber.scope),
                scope: Some(fiber.scope),
                priority: fiber.order.priority,
                phase: FiberPhase::RunScope,
            },
            Work::DiffComponentProps(diff) => {
                let scope = diff.updates.first().map(|update| update.scope);
                FiberInfo {
                    id: scope.and_then(|scope| self.fiber_id_for_scope(scope)),
                    scope,
                    priority: diff.priority,
                    phase: FiberPhase::Diff,
                }
            }
            Work::PollTask(task) => {
                let scope = self.runtime.task_scope(*task);
                FiberInfo {
                    id: scope.and_then(|scope| self.fiber_id_for_scope(scope)),
                    scope,
                    priority: UpdatePriority::Default,
                    phase: FiberPhase::PollTask,
                }
            }
            Work::RunEffect(effect) => FiberInfo {
                id: self.fiber_id_for_scope(effect.order.id),
                scope: Some(effect.order.id),
                priority: UpdatePriority::Idle,
                phase: FiberPhase::Effect,
            },
        }
    }

    fn fiber_id_for_scope(&self, scope: ScopeId) -> Option<FiberId> {
        let mount = self
            .scopes
            .get(scope.0)
            .and_then(|scope| scope.last_rendered_node.as_ref())
            .and_then(|node| node.mount.get().as_usize())?;

        self.runtime
            .fibers
            .borrow()
            .get(mount)
            .map(|fiber| fiber.id)
    }
}
