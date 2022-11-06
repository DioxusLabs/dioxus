use std::{ops::DerefMut, pin::Pin};

use futures_task::Context;
use futures_util::StreamExt;

use crate::{innerlude::make_task_waker, VirtualDom};

use super::SchedulerMsg;

impl VirtualDom {
    /// Wait for futures internal to the virtualdom
    ///
    /// This is cancel safe, so if the future is dropped, you can push events into the virtualdom
    pub async fn wait_for_work(&mut self) {
        loop {
            let msg = self.scheduler.rx.next().await.unwrap();

            println!("msg received: {:?}", msg);

            match msg {
                SchedulerMsg::Event => todo!(),
                SchedulerMsg::Immediate(_) => todo!(),
                SchedulerMsg::DirtyAll => todo!(),
                SchedulerMsg::TaskNotified(id) => {
                    let mut tasks = self.scheduler.handle.tasks.borrow_mut();
                    let local_task = &tasks[id.0];

                    // // attach the waker to itself
                    let waker = make_task_waker(local_task);
                    let mut cx = Context::from_waker(&waker);
                    let mut fut = unsafe { &mut *local_task.task };

                    let pinned = unsafe { Pin::new_unchecked(fut.deref_mut()) };

                    match pinned.poll(&mut cx) {
                        futures_task::Poll::Ready(_) => {
                            // remove the task
                            tasks.remove(id.0);
                        }
                        futures_task::Poll::Pending => {}
                    }

                    if tasks.is_empty() {
                        return;
                    }
                }
                // SchedulerMsg::TaskNotified(id) => {},
                SchedulerMsg::SuspenseNotified(_) => todo!(),
            }
        }
    }
}
