use crate::innerlude::*;
use futures_channel::mpsc::UnboundedSender;

pub struct TaskHandle {
    pub(crate) sender: UnboundedSender<SchedulerMsg>,
    pub(crate) our_id: u64,
}

impl TaskHandle {
    /// Toggles this coroutine off/on.
    ///
    /// This method is not synchronous - your task will not stop immediately.
    pub fn toggle(&self) {
        self.sender
            .unbounded_send(SchedulerMsg::ToggleTask(self.our_id))
            .unwrap()
    }

    /// This method is not synchronous - your task will not stop immediately.
    pub fn resume(&self) {
        self.sender
            .unbounded_send(SchedulerMsg::ResumeTask(self.our_id))
            .unwrap()
    }

    /// This method is not synchronous - your task will not stop immediately.
    pub fn stop(&self) {
        self.sender
            .unbounded_send(SchedulerMsg::ToggleTask(self.our_id))
            .unwrap()
    }

    /// This method is not synchronous - your task will not stop immediately.
    pub fn restart(&self) {
        self.sender
            .unbounded_send(SchedulerMsg::ToggleTask(self.our_id))
            .unwrap()
    }
}
