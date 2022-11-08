use crate::{innerlude::SuspenseContext, scheduler::RcWake};
use futures_util::{pin_mut, task::noop_waker_ref};
use std::{
    mem,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use crate::{
    any_props::AnyProps,
    arena::ElementId,
    bump_frame::BumpFrame,
    factory::RenderReturn,
    innerlude::{SuspenseId, SuspenseLeaf},
    scopes::{ScopeId, ScopeState},
    virtualdom::VirtualDom,
};

impl VirtualDom {
    pub fn new_scope(&mut self, props: *mut dyn AnyProps<'static>) -> ScopeId {
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
            render_cnt: Default::default(),
            hook_arena: Default::default(),
            hook_vals: Default::default(),
            hook_idx: Default::default(),
            shared_contexts: Default::default(),
            tasks: self.scheduler.handle.clone(),
        });

        id
    }

    pub fn acquire_current_container(&self) -> ElementId {
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

    pub fn run_scope(&mut self, scope_id: ScopeId) -> &mut RenderReturn {
        let mut new_nodes = unsafe {
            let scope = &mut self.scopes[scope_id.0];
            scope.hook_idx.set(0);

            let props: &mut dyn AnyProps = mem::transmute(&mut *scope.props);
            let res: RenderReturn = props.render(scope);
            let res: RenderReturn<'static> = mem::transmute(res);
            res
        };

        // immediately resolve futures that can be resolved
        if let RenderReturn::Async(task) = &mut new_nodes {
            use futures_util::FutureExt;

            let mut leaves = self.scheduler.handle.leaves.borrow_mut();
            let entry = leaves.vacant_entry();
            let key = entry.key();

            let leaf = Rc::new(SuspenseLeaf {
                scope_id,
                task: task.as_mut(),
                id: SuspenseId(key),
                tx: self.scheduler.handle.sender.clone(),
                boundary: ScopeId(0),
                notified: false.into(),
            });

            let _leaf = leaf.clone();
            let waker = leaf.waker();
            let mut cx = Context::from_waker(&waker);
            let mut pinned = unsafe { Pin::new_unchecked(task.as_mut()) };

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
                    _ if _leaf.notified.get() => {
                        _leaf.notified.set(false);
                        continue;
                    }

                    // If no nodes are produced, then we need to wait for the future to be woken up
                    // Insert the future into fiber leaves and break
                    _ => {
                        entry.insert(leaf);
                        break;
                    }
                };
            }
        };

        let scope = &mut self.scopes[scope_id.0];
        let frame = match scope.render_cnt % 2 {
            0 => &mut scope.node_arena_1,
            1 => &mut scope.node_arena_2,
            _ => unreachable!(),
        };

        // set the head of the bump frame
        let alloced = frame.bump.alloc(new_nodes);
        frame.node.set(alloced);

        // rebind the lifetime now that its stored internally
        unsafe { mem::transmute(alloced) }
    }
}
