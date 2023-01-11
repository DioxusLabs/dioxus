use crate::{
    any_props::AnyProps,
    bump_frame::BumpFrame,
    innerlude::DirtyScope,
    innerlude::{SuspenseHandle, SuspenseId, SuspenseLeaf},
    nodes::RenderReturn,
    scopes::{ScopeId, ScopeState},
    virtual_dom::VirtualDom,
};
use futures_util::FutureExt;
use std::{
    mem,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

impl VirtualDom {
    pub(super) fn new_scope(
        &mut self,
        props: Box<dyn AnyProps<'static>>,
        name: &'static str,
    ) -> &ScopeState {
        let parent = self.acquire_current_scope_raw();
        let entry = self.scopes.vacant_entry();
        let height = unsafe { parent.map(|f| (*f).height + 1).unwrap_or(0) };
        let id = ScopeId(entry.key());

        entry.insert(Box::new(ScopeState {
            parent,
            id,
            height,
            name,
            props: Some(props),
            tasks: self.scheduler.clone(),
            placeholder: Default::default(),
            node_arena_1: BumpFrame::new(0),
            node_arena_2: BumpFrame::new(0),
            spawned_tasks: Default::default(),
            render_cnt: Default::default(),
            hook_arena: Default::default(),
            hook_list: Default::default(),
            hook_idx: Default::default(),
            shared_contexts: Default::default(),
            borrowed_props: Default::default(),
            attributes_to_drop: Default::default(),
        }))
    }

    fn acquire_current_scope_raw(&self) -> Option<*const ScopeState> {
        let id = self.scope_stack.last().copied()?;
        let scope = self.scopes.get(id.0)?;
        Some(scope.as_ref())
    }

    pub(crate) fn run_scope(&mut self, scope_id: ScopeId) -> &RenderReturn {
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        // Remove all the outdated listeners
        self.ensure_drop_safety(scope_id);

        let mut new_nodes = unsafe {
            self.scopes[scope_id.0].previous_frame().bump_mut().reset();

            let scope = &self.scopes[scope_id.0];

            scope.hook_idx.set(0);

            // safety: due to how we traverse the tree, we know that the scope is not currently aliased
            let props: &dyn AnyProps = scope.props.as_ref().unwrap().as_ref();
            let props: &dyn AnyProps = mem::transmute(props);

            props.render(scope).extend_lifetime()
        };

        // immediately resolve futures that can be resolved
        if let RenderReturn::Pending(task) = &mut new_nodes {
            let mut leaves = self.scheduler.leaves.borrow_mut();

            let entry = leaves.vacant_entry();
            let suspense_id = SuspenseId(entry.key());

            let leaf = SuspenseLeaf {
                scope_id,
                task: task.as_mut(),
                notified: Default::default(),
                waker: futures_util::task::waker(Arc::new(SuspenseHandle {
                    id: suspense_id,
                    tx: self.scheduler.sender.clone(),
                })),
            };

            let mut cx = Context::from_waker(&leaf.waker);

            // safety: the task is already pinned in the bump arena
            let mut pinned = unsafe { Pin::new_unchecked(task.as_mut()) };

            // Keep polling until either we get a value or the future is not ready
            loop {
                match pinned.poll_unpin(&mut cx) {
                    // If nodes are produced, then set it and we can break
                    Poll::Ready(nodes) => {
                        new_nodes = match nodes {
                            Some(nodes) => RenderReturn::Ready(nodes),
                            None => RenderReturn::default(),
                        };

                        break;
                    }

                    // If no nodes are produced but the future woke up immediately, then try polling it again
                    // This circumvents things like yield_now, but is important is important when rendering
                    // components that are just a stream of immediately ready futures
                    _ if leaf.notified.get() => {
                        leaf.notified.set(false);
                        continue;
                    }

                    // If no nodes are produced, then we need to wait for the future to be woken up
                    // Insert the future into fiber leaves and break
                    _ => {
                        entry.insert(leaf);
                        self.collected_leaves.push(suspense_id);
                        break;
                    }
                };
            }
        };

        let scope = &self.scopes[scope_id.0];

        // We write on top of the previous frame and then make it the current by pushing the generation forward
        let frame = scope.previous_frame();

        // set the new head of the bump frame
        let allocated = &*frame.bump().alloc(new_nodes);
        frame.node.set(allocated);

        // And move the render generation forward by one
        scope.render_cnt.set(scope.render_cnt.get() + 1);

        // remove this scope from dirty scopes
        self.dirty_scopes.remove(&DirtyScope {
            height: scope.height,
            id: scope.id,
        });

        // rebind the lifetime now that its stored internally
        unsafe { allocated.extend_lifetime_ref() }
    }
}
