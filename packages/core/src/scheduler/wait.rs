use futures_task::Context;
use futures_util::{FutureExt, StreamExt};

use crate::{
    factory::RenderReturn,
    innerlude::{Mutation, Renderer, SuspenseContext},
    VNode, VirtualDom,
};

use super::{waker::RcWake, SchedulerMsg, SuspenseLeaf};

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
                    let waker = local_task.waker();
                    let mut cx = Context::from_waker(&waker);

                    // safety: the waker owns its task and everythig is single threaded
                    let fut = unsafe { &mut *local_task.task.get() };

                    if let futures_task::Poll::Ready(_) = fut.poll_unpin(&mut cx) {
                        tasks.remove(id.0);
                    }
                }

                SchedulerMsg::SuspenseNotified(id) => {
                    println!("suspense notified");

                    let leaf = self
                        .scheduler
                        .handle
                        .leaves
                        .borrow_mut()
                        .get(id.0)
                        .unwrap()
                        .clone();

                    let scope_id = leaf.scope_id;

                    // todo: cache the waker
                    let waker = leaf.waker();
                    let mut cx = Context::from_waker(&waker);

                    let fut = unsafe { &mut *leaf.task };

                    let mut pinned = unsafe { std::pin::Pin::new_unchecked(fut) };
                    let as_pinned_mut = &mut pinned;

                    // the component finished rendering and gave us nodes
                    // we should attach them to that component and then render its children
                    // continue rendering the tree until we hit yet another suspended component
                    if let futures_task::Poll::Ready(new_nodes) = as_pinned_mut.poll_unpin(&mut cx)
                    {
                        let boundary = &self.scopes[leaf.scope_id.0]
                            .consume_context::<SuspenseContext>()
                            .unwrap();

                        println!("ready pool");

                        let mut fiber = boundary.borrow_mut();

                        println!(
                            "Existing mutations {:?}, scope {:?}",
                            fiber.mutations, fiber.id
                        );

                        let scope = &mut self.scopes[scope_id.0];
                        let arena = scope.current_arena();

                        let ret = arena.bump.alloc(RenderReturn::Sync(new_nodes));
                        arena.node.set(ret);

                        if let RenderReturn::Sync(Some(template)) = ret {
                            let mutations = &mut fiber.mutations;
                            let template: &VNode = unsafe { std::mem::transmute(template) };
                            let mutations: &mut Renderer =
                                unsafe { std::mem::transmute(mutations) };

                            self.scope_stack.push(scope_id);
                            self.create(mutations, template);
                            self.scope_stack.pop();

                            println!("{:#?}", mutations);
                        } else {
                            println!("nodes arent right");
                        }
                    } else {
                        println!("not ready");
                    }
                }
            }

            // now proces any events. If we end up running a component and it generates mutations, then we should run those mutations
        }
    }
}
