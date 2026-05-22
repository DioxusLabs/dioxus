use super::{ComponentPropsDiff, ScopeOrder, UpdatePriority};
use crate::{
    Task, VirtualDom,
    innerlude::{Effect, WriteMutations},
    scopes::ScopeId,
};

#[derive(Clone, Copy)]
pub(crate) enum WorkCandidate {
    Fiber,
    Task,
    Fragment(usize),
}

#[derive(Debug)]
pub(crate) enum Work {
    DiffFiber(DirtyFiber),
    DiffComponentProps(ComponentPropsDiff),
    PollTask(Task),
    RunEffect(Effect),
}

impl Work {
    pub(crate) fn priority(&self) -> UpdatePriority {
        match self {
            Self::DiffFiber(fiber) => fiber.order.priority,
            Self::DiffComponentProps(diff) => diff.priority,
            Self::PollTask(_) => UpdatePriority::Default,
            Self::RunEffect(_) => UpdatePriority::Idle,
        }
    }

    pub(crate) fn requires_commit(&self) -> bool {
        !matches!(self, Self::RunEffect(_))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DirtyFiber {
    pub(crate) scope: ScopeId,
    pub(crate) order: ScopeOrder,
}

impl DirtyFiber {
    pub(crate) fn new(order: ScopeOrder) -> Self {
        Self {
            scope: order.id,
            order,
        }
    }

    pub(crate) fn diff_into<M: WriteMutations>(
        self,
        dom: &mut VirtualDom,
        to: Option<&mut M>,
        priority: UpdatePriority,
    ) {
        tracing::trace!(
            ?self.scope,
            height = self.order.height,
            "Diffing dirty fiber"
        );
        let previous = dom.render_priority;
        let previous_deferred_priority = dom.render_deferred_priority;
        dom.render_priority = priority;
        dom.render_deferred_priority = dom
            .dirty_fibers
            .deferred_priority_for_scope(self.scope, priority);
        dom.runtime
            .clone()
            .with_update_priority(priority, || dom.run_and_diff_scope(to, self.scope));
        dom.render_priority = previous;
        dom.render_deferred_priority = previous_deferred_priority;
    }
}
