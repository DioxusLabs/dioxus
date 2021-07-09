//! The TaskQueue serves as a centralized async store for all tasks in Dioxus.
//! When a component renders, it may submit an async task to the queue.
//!
//! Then the task complete, it is emitted from the virtual dom in the event loop, which is then fed back into the virtualdom
//! as an event trigger.
//!
//! When a component is scheduled to re-render, the awaing task must be dumped from the queue.
//!
//! This is all pretty unsafe stuff.
//! The major invariant here is that tasks that enter the queue may be invalidated during transitions.

use std::pin::Pin;

use futures::{Future, Stream, StreamExt};
use slotmap::{DefaultKey, SlotMap};

use crate::events::EventTrigger;

pub struct TaskQueue {
    slots: SlotMap<DefaultKey, Task>,
}

impl TaskQueue {
    unsafe fn push_task(&mut self, task: Task) -> TaskHandle {
        todo!()
    }

    async fn next(&mut self) -> EventTrigger {
        for (key, task) in self.slots.iter_mut() {
            let ptr = task.0;
        }
        todo!()
    }
}

struct Task(*mut Pin<Box<dyn Future<Output = ()>>>);

struct TaskHandle {}
