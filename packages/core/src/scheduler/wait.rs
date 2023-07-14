use crate::{innerlude::SuspenseContext, ScopeId, TaskId, VirtualDom};
use std::{rc::Rc, task::Context};

impl VirtualDom {
    /// Handle notifications by tasks inside the scheduler
    ///
    /// This is precise, meaning we won't poll every task, just tasks that have woken up as notified to use by the
    /// queue
    pub(crate) fn handle_task_wakeup(&mut self, id: TaskId) {
        let mut tasks = self.scheduler.tasks.borrow_mut();

        let task = match tasks.get(id.0) {
            Some(task) => task,
            // The task was removed from the scheduler, so we can just ignore it
            None => return,
        };

        let mut cx = Context::from_waker(&task.waker);

        // If the task completes...
        if task.task.borrow_mut().as_mut().poll(&mut cx).is_ready() {
            // Remove it from the scope so we dont try to double drop it when the scope dropes
            let scope = &self.scopes[task.scope];
            scope.spawned_tasks.borrow_mut().remove(&id);

            // Remove it from the scheduler
            tasks.try_remove(id.0);
        }
    }

    pub(crate) fn acquire_suspense_boundary(&self, id: ScopeId) -> Rc<SuspenseContext> {
        self.scopes[id]
            .consume_context::<Rc<SuspenseContext>>()
            .unwrap()
    }
}
