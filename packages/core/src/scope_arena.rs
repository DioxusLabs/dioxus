use crate::{
    any_props::AnyProps,
    arena::ElementId,
    bump_frame::BumpFrame,
    factory::RenderReturn,
    innerlude::{SuspenseId, SuspenseLeaf},
    scheduler::RcWake,
    scopes::{ScopeId, ScopeState},
    virtual_dom::VirtualDom,
};
use futures_util::FutureExt;
use std::{
    mem,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

impl VirtualDom {
    pub(super) fn new_scope(&mut self, props: *mut dyn AnyProps<'static>) -> &mut ScopeState {
        let parent = self.acquire_current_scope_raw();
        let container = self.acquire_current_container();
        let entry = self.scopes.vacant_entry();
        let height = unsafe { parent.map(|f| (*f).height).unwrap_or(0) + 1 };
        let id = ScopeId(entry.key());

        entry.insert(ScopeState {
            parent,
            container,
            id,
            height,
            props,
            placeholder: None.into(),
            node_arena_1: BumpFrame::new(50),
            node_arena_2: BumpFrame::new(50),
            spawned_tasks: Default::default(),
            render_cnt: Default::default(),
            hook_arena: Default::default(),
            hook_vals: Default::default(),
            hook_idx: Default::default(),
            shared_contexts: Default::default(),
            tasks: self.scheduler.clone(),
        })
    }

    fn acquire_current_container(&self) -> ElementId {
        self.element_stack
            .last()
            .copied()
            .expect("Always have a container")
    }

    fn acquire_current_scope_raw(&mut self) -> Option<*mut ScopeState> {
        self.scope_stack
            .last()
            .copied()
            .and_then(|id| self.scopes.get_mut(id.0).map(|f| f as *mut ScopeState))
    }

    pub(crate) unsafe fn run_scope_extend<'a>(
        &mut self,
        scope_id: ScopeId,
    ) -> &'a RenderReturn<'a> {
        unsafe { self.run_scope(scope_id).extend_lifetime_ref() }
    }

    pub(crate) fn run_scope(&mut self, scope_id: ScopeId) -> &RenderReturn {
        println!("run_scope: {:?}", scope_id);

        let mut new_nodes = unsafe {
            let scope = &mut self.scopes[scope_id.0];
            println!("run_scope: scope: {:?}", scope.render_cnt.get());
            scope.hook_idx.set(0);

            // safety: due to how we traverse the tree, we know that the scope is not currently aliased
            let props: &mut dyn AnyProps = mem::transmute(&mut *scope.props);
            props.render(scope).extend_lifetime()
        };

        // immediately resolve futures that can be resolved
        if let RenderReturn::Async(task) = &mut new_nodes {
            let mut leaves = self.scheduler.leaves.borrow_mut();

            let entry = leaves.vacant_entry();
            let suspense_id = SuspenseId(entry.key());

            let leaf = Rc::new(SuspenseLeaf {
                scope_id,
                task: task.as_mut(),
                id: suspense_id,
                tx: self.scheduler.sender.clone(),
                notified: Default::default(),
            });

            let waker = leaf.waker();
            let mut cx = Context::from_waker(&waker);

            // safety: the task is already pinned in the bump arena
            let mut pinned = unsafe { Pin::new_unchecked(task.as_mut()) };

            // Keep polling until either we get a value or the future is not ready
            loop {
                match pinned.poll_unpin(&mut cx) {
                    // If nodes are produced, then set it and we can break
                    Poll::Ready(nodes) => {
                        new_nodes = RenderReturn::Sync(nodes);
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

        /*
        todo: use proper mutability here

        right now we're aliasing the scope, which is not allowed
        */

        let scope = &mut self.scopes[scope_id.0];
        let frame = scope.current_frame();

        // set the head of the bump frame
        let alloced = frame.bump.alloc(new_nodes);
        frame.node.set(alloced);

        // And move the render generation forward by one
        scope.render_cnt.set(scope.render_cnt.get() + 1);

        // rebind the lifetime now that its stored internally
        unsafe { mem::transmute(alloced) }
    }
}
