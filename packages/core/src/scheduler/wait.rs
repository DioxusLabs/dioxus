use futures_task::Context;
use futures_util::{FutureExt, StreamExt};

use crate::{innerlude::make_task_waker, VirtualDom};

use super::SchedulerMsg;

impl VirtualDom {
    /// Wait for futures internal to the virtualdom
    ///
    /// This is cancel safe, so if the future is dropped, you can push events into the virtualdom
    pub async fn wait_for_work(&mut self) {
        loop {
            match self.scheduler.rx.next().await.unwrap() {
                SchedulerMsg::Event => todo!(),
                SchedulerMsg::Immediate(_) => todo!(),
                SchedulerMsg::DirtyAll => todo!(),

                SchedulerMsg::TaskNotified(id) => {
                    let mut tasks = self.scheduler.handle.tasks.borrow_mut();
                    let local_task = &tasks[id.0];

                    // attach the waker to itself
                    // todo: don't make a new waker every time, make it once and then just clone it
                    let waker = make_task_waker(local_task.clone());
                    let mut cx = Context::from_waker(&waker);

                    // safety: the waker owns its task and everythig is single threaded
                    let fut = unsafe { &mut *local_task.task.get() };

                    if let futures_task::Poll::Ready(_) = fut.poll_unpin(&mut cx) {
                        tasks.remove(id.0);
                    }
                }

                SchedulerMsg::SuspenseNotified(_) => todo!(),
            }
        }
    }
}
