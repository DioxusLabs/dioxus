use crate::{runtime::RuntimeGuard, TaskId, VirtualDom};
use std::task::Context;

impl VirtualDom {
    /// Handle notifications by tasks inside the scheduler
    ///
    /// This is precise, meaning we won't poll every task, just tasks that have woken up as notified to use by the
    /// queue
    pub(crate) fn handle_task_wakeup(&mut self, id: TaskId) {
        let _runtime = RuntimeGuard::new(self.runtime.clone());
        let mut tasks = self.runtime.scheduler.tasks.borrow_mut();

        let task = match tasks.get(id.0) {
            Some(task) => task,
            // The task was removed from the scheduler, so we can just ignore it
            None => return,
        };

        let mut cx = Context::from_waker(&task.waker);

        // update the scope stack
        self.runtime.scope_stack.borrow_mut().push(task.scope);
        self.runtime.rendering.set(false);

        // If the task completes...
        if task.task.borrow_mut().as_mut().poll(&mut cx).is_ready() {
            // Remove it from the scope so we dont try to double drop it when the scope dropes
            let scope = &self.get_scope(task.scope).unwrap();
            scope.context().spawned_tasks.borrow_mut().remove(&id);

            // Remove it from the scheduler
            tasks.try_remove(id.0);
        }

        // Remove the scope from the stack
        self.runtime.scope_stack.borrow_mut().pop();
        self.runtime.rendering.set(true);
    }
}
