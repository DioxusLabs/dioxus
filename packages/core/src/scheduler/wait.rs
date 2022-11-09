use futures_util::{FutureExt, StreamExt};
use std::task::{Context, Poll};

use crate::{
    diff::DirtyScope,
    factory::RenderReturn,
    innerlude::{Mutation, Mutations, SuspenseContext},
    VNode, VirtualDom,
};

use super::{waker::RcWake, SchedulerMsg, SuspenseLeaf};

impl VirtualDom {
    /// Wait for futures internal to the virtualdom
    ///
    /// This is cancel safe, so if the future is dropped, you can push events into the virtualdom
    pub async fn wait_for_work(&mut self) {
        // todo: make sure the scheduler queue is completely drained
        loop {
            match self.rx.next().await.unwrap() {
                SchedulerMsg::Event => break,

                SchedulerMsg::Immediate(id) => {
                    let height = self.scopes[id.0].height;
                    self.dirty_scopes.insert(DirtyScope { height, id });
                    break;
                }

                SchedulerMsg::DirtyAll => todo!(),

                SchedulerMsg::TaskNotified(id) => {
                    let mut tasks = self.scheduler.tasks.borrow_mut();
                    let task = &tasks[id.0];

                    // If the task completes...
                    if task.progress() {
                        // Remove it from the scope so we dont try to double drop it when the scope dropes
                        self.scopes[task.scope.0].spawned_tasks.remove(&id);

                        // Remove it from the scheduler
                        tasks.remove(id.0);
                    }
                }

                SchedulerMsg::SuspenseNotified(id) => {
                    println!("suspense notified");

                    let leaf = self
                        .scheduler
                        .leaves
                        .borrow_mut()
                        .get(id.0)
                        .unwrap()
                        .clone();

                    let scope_id = leaf.scope_id;

                    // todo: cache the waker
                    let waker = leaf.waker();
                    let mut cx = Context::from_waker(&waker);

                    // Safety: the future is always pinned to the bump arena
                    let mut pinned = unsafe { std::pin::Pin::new_unchecked(&mut *leaf.task) };
                    let as_pinned_mut = &mut pinned;

                    // the component finished rendering and gave us nodes
                    // we should attach them to that component and then render its children
                    // continue rendering the tree until we hit yet another suspended component
                    if let Poll::Ready(new_nodes) = as_pinned_mut.poll_unpin(&mut cx) {
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
                            let mutations: &mut Mutations =
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
        }
    }
}
