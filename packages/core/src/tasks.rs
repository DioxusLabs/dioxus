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

use std::{
    cell::Cell,
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};

use futures_util::{stream::FuturesUnordered, Future, Stream, StreamExt};
use slotmap::{DefaultKey, SlotMap};

use crate::innerlude::{EventTrigger, FiberTask, ScopeIdx};

pub type TaskSubmitter = Arc<dyn Fn(FiberTask)>;

pub struct TaskQueue {
    slots: Arc<RwLock<FuturesUnordered<FiberTask>>>,
    // slots: Arc<RwLock<SlotMap<DefaultKey, DTask>>>,
    submitter: TaskSubmitter,
}

impl TaskQueue {
    pub fn new() -> Self {
        let slots = Arc::new(RwLock::new(FuturesUnordered::new()));
        let slots2 = slots.clone();

        let submitter = Arc::new(move |task| {
            let mut slots = slots2.write().unwrap();
            log::debug!("Task submitted into global task queue");
            slots.push(task);
        });
        Self { slots, submitter }
    }

    pub fn new_submitter(&self) -> TaskSubmitter {
        self.submitter.clone()
    }

    pub fn submit_task(&mut self, task: FiberTask) {
        self.slots.write().unwrap().push(task);
        // TaskHandle { key }
    }

    pub fn is_empty(&self) -> bool {
        self.slots.read().unwrap().is_empty()
    }
    pub fn len(&self) -> usize {
        self.slots.read().unwrap().len()
    }

    pub async fn next(&mut self) -> Option<EventTrigger> {
        let mut slots = self.slots.write().unwrap();
        slots.next().await
    }
}

// impl Stream for TaskQueue {
//     type Item = EventTrigger;

//     /// We can never be finished polling
//     fn poll_next(
//         self: Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Option<Self::Item>> {
//         // let yield_every = self.len();
//         // let mut polled = 0;

//         let mut slots = self.slots.write().unwrap();
//         for (_key, slot) in slots.iter_mut() {
//             if slot.dead.get() {
//                 continue;
//             }
//             let r = slot.fut;
//             // let fut = unsafe { &mut *r };
//             // use futures::{future::Future, poll, FutureExt};

//             let f2 = fut.as_mut();
//             let w = cx.waker();
//             let mut cx = Context::from_waker(&w);

//             // Pin::new_unchecked(pointer)
//             // use std::future::Future;
//             match f2.poll(&mut cx) {
//                 Poll::Ready(_) => {
//                     let trigger = EventTrigger::new_from_task(slot.originator);
//                     slot.dead.set(true);
//                     return Poll::Ready(Some(trigger));
//                 }
//                 Poll::Pending => continue,
//             }
//         }

//         // we tried polling every active task.
//         // give up and relinquish controlto the parent

//         // We have polled a large number of futures in a row without yielding.
//         // To ensure we do not starve other tasks waiting on the executor,
//         // we yield here, but immediately wake ourselves up to continue.
//         // cx.waker().wake_by_ref();
//         return Poll::Pending;
//     }
// }

pub struct TaskHandle {
    key: DefaultKey,
}

pub struct DTask {
    fut: FiberTask,
    originator: ScopeIdx,
    dead: Cell<bool>,
}
impl DTask {
    pub fn new(fut: FiberTask, originator: ScopeIdx) -> Self {
        Self {
            fut,
            originator,
            dead: Cell::new(false),
        }
    }
    pub fn debug_new(fut: FiberTask) -> Self {
        let originator = ScopeIdx::default();
        Self {
            fut,
            originator,
            dead: Cell::new(false),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use std::time::Duration;

//     use super::*;
//     use bumpalo::Bump;

//     #[async_std::test]
//     async fn example() {
//         let bump = Bump::new();
//         type RawTask = Pin<Box<dyn Future<Output = ()>>>;
//         // build the three
//         let f1 = bump.alloc(Box::pin(async {
//             //
//             async_std::task::sleep(Duration::from_secs(3)).await;
//             println!("3 sec")
//         }) as RawTask);

//         let f2 = bump.alloc(Box::pin(async {
//             //
//             async_std::task::sleep(Duration::from_secs(2)).await;
//             println!("2 sec")
//         }) as RawTask);

//         let f3 = bump.alloc(Box::pin(async {
//             //
//             async_std::task::sleep(Duration::from_secs(1)).await;
//             println!("1 sec");
//         }) as RawTask);

//         let mut queue = TaskQueue::new();
//         queue.submit_task(DTask::debug_new(f1));
//         queue.submit_task(DTask::debug_new(f2));
//         queue.submit_task(DTask::debug_new(f3));

//         while !queue.is_empty() {
//             let next = queue.next().await;
//             println!("Event received {:#?}", next);
//         }
//     }
// }
