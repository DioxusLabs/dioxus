//! Container for polling suspended nodes
//!
//! Whenever a future is returns a value, we walk the tree upwards and check if any of the parents are suspended.

use bumpalo::boxed::Box as BumpBox;
use futures_util::Future;
use std::{
    collections::{HashMap, HashSet},
    future::poll_fn,
    pin::Pin,
    task::Poll,
};

use crate::{
    factory::FiberLeaf, innerlude::Mutation, Element, ElementId, ScopeId, VNode, VirtualDom,
};

impl VirtualDom {
    // todo: lots of hammering lifetimes here...
    async fn wait_for_suspense(&mut self) {
        let res = poll_fn(|cx| {
            let all_suspended_complete = true;

            let suspended_scopes: Vec<_> = self.suspended_scopes.iter().copied().collect();

            for scope in suspended_scopes {
                let mut fiber = self.scopes[scope.0]
                    .suspense_boundary
                    .as_mut()
                    .expect(" A fiber to be present if the scope is suspended");

                let mut fiber: &mut Fiber = unsafe { std::mem::transmute(fiber) };

                let mutations = &mut fiber.mutations;
                let mutations: &mut Vec<Mutation> = unsafe { std::mem::transmute(mutations) };

                let keys = fiber.futures.keys().copied().collect::<Vec<_>>();
                for loc in keys {
                    let fut = *fiber.futures.get_mut(&loc).unwrap();
                    let fut = unsafe { &mut *fut };
                    let fut: &mut FiberLeaf<'_> = unsafe { std::mem::transmute(fut) };

                    use futures_util::FutureExt;

                    match fut.poll_unpin(cx) {
                        Poll::Ready(nodes) => {
                            // remove the future from the fiber
                            fiber.futures.remove(&loc).unwrap();

                            // set the original location to the new nodes
                            // todo!("set the original location to the new nodes");
                            let template = nodes.unwrap();

                            let scope = &self.scopes[scope.0];
                            let template = scope.bump().alloc(template);
                            let template: &VNode = unsafe { std::mem::transmute(template) };

                            // now create the template
                            self.create(mutations, template);
                        }
                        Poll::Pending => todo!("still working huh"),
                    }
                }

                // let mut fiber = Pin::new(&mut fiber);

                // let mut scope = scope;
                // let mut vnode = self.scopes[scope.0].vnode.take().unwrap();

                // let mut vnode = Pin::new(&mut vnode);

                // let mut vnode = poll_fn(|cx| {
                //     let mut vnode = Pin::new(&mut vnode);
                //     let mut fiber = Pin::new(&mut fiber);

                //     let res = vnode.as_mut().poll(cx);

                //     if let Poll::Ready(res) = res {
                //         Poll::Ready(res)
                //     } else {
                //         Poll::Pending
                //     }
                // })
                // .await;

                // self.scopes[scope.0].vnode = Some(vnode);
                // self.scopes[scope.0].suspense_boundary = Some(fiber);
            }

            match all_suspended_complete {
                true => Poll::Ready(()),
                false => Poll::Pending,
            }
        });

        todo!()
    }
}

// impl SuspenseGenerator {
//     async fn wait_for_work(&mut self) {
//         use futures_util::future::{select, Either};

//         // let scopes = &mut self.scopes;
//         let suspense_status = poll_fn(|cx| {
//             // let mut tasks = scopes.tasks.tasks.borrow_mut();
//             // tasks.retain(|_, task| task.as_mut().poll(cx).is_pending());

//             match true {
//                 // match tasks.is_empty() {
//                 true => Poll::Ready(()),
//                 false => Poll::Pending,
//             }
//         });

//         // Suspense {
//         // maybe generate futures
//         // only render when all the futures are ready
//         // }

//         /*
//             div {
//                 as1 {}
//                 as2 {}
//                 as3 {}
//             }
//         */
//         // match select(task_poll, self.channel.1.next()).await {
//         //     Either::Left((_, _)) => {}
//         //     Either::Right((msg, _)) => self.pending_messages.push_front(msg.unwrap()),
//         // }
//     }
// }

#[derive(Default)]
pub struct Fiber<'a> {
    // The work-in progress of this suspended tree
    pub mutations: Vec<Mutation<'a>>,

    // All the pending futures (DFS)
    pub futures:
        HashMap<LeafLocation, *mut Pin<BumpBox<'a, dyn Future<Output = Element<'a>> + 'a>>>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct LeafLocation {
    pub scope: ScopeId,
    pub element: ElementId,
}
