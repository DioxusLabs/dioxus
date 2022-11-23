use futures_util::FutureExt;
use std::task::{Context, Poll};

use crate::{
    factory::RenderReturn,
    innerlude::{Mutation, Mutations, SuspenseContext},
    TaskId, VNode, VirtualDom,
};

use super::{waker::RcWake, SuspenseId};

impl VirtualDom {
    /// Handle notifications by tasks inside the scheduler
    ///
    /// This is precise, meaning we won't poll every task, just tasks that have woken up as notified to use by the
    /// queue
    pub fn handle_task_wakeup(&mut self, id: TaskId) {
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

    pub fn handle_suspense_wakeup(&mut self, id: SuspenseId) {
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
            let fiber = &self.scopes[leaf.scope_id.0]
                .consume_context::<SuspenseContext>()
                .unwrap();

            println!("ready pool");

            println!(
                "Existing mutations {:?}, scope {:?}",
                fiber.mutations, fiber.id
            );

            let scope = &mut self.scopes[scope_id.0];
            let arena = scope.current_frame();

            let ret = arena.bump.alloc(RenderReturn::Sync(new_nodes));
            arena.node.set(ret);

            fiber.waiting_on.borrow_mut().remove(&id);

            if let RenderReturn::Sync(Ok(template)) = ret {
                let mutations_ref = &mut fiber.mutations.borrow_mut();
                let mutations = &mut **mutations_ref;
                let template: &VNode = unsafe { std::mem::transmute(template) };
                let mutations: &mut Mutations = unsafe { std::mem::transmute(mutations) };

                let place_holder_id = scope.placeholder.get().unwrap();
                self.scope_stack.push(scope_id);
                let created = self.create(mutations, template);
                self.scope_stack.pop();
                mutations.push(Mutation::ReplaceWith {
                    id: place_holder_id,
                    m: created,
                });

                // for leaf in self.collected_leaves.drain(..) {
                //     fiber.waiting_on.borrow_mut().insert(leaf);
                // }

                if fiber.waiting_on.borrow().is_empty() {
                    println!("fiber is finished!");
                    self.finished_fibers.push(fiber.id);
                } else {
                    println!("fiber is not finished {:?}", fiber.waiting_on);
                }
            } else {
                println!("nodes arent right");
            }
        } else {
            println!("not ready");
        }
    }
}
