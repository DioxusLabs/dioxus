use std::task::Context;

use futures_util::task::noop_waker_ref;

use crate::{
    any_props::AnyProps,
    arena::ElementId,
    bump_frame::BumpFrame,
    factory::RenderReturn,
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

    pub fn run_scope(&mut self, id: ScopeId) -> &mut RenderReturn {
        let scope = &mut self.scopes[id.0];
        scope.hook_idx.set(0);

        let mut new_nodes = {
            let props = unsafe { &mut *scope.props };
            let props: &mut dyn AnyProps = unsafe { std::mem::transmute(props) };
            let res: RenderReturn = props.render(scope);
            let res: RenderReturn<'static> = unsafe { std::mem::transmute(res) };
            res
        };

        // immediately resolve futures that can be resolved immediatelys
        let res = match &mut new_nodes {
            RenderReturn::Sync(_) => new_nodes,
            RenderReturn::Async(fut) => {
                use futures_util::FutureExt;
                let mut cx = Context::from_waker(&noop_waker_ref());

                match fut.poll_unpin(&mut cx) {
                    std::task::Poll::Ready(nodes) => RenderReturn::Sync(nodes),
                    std::task::Poll::Pending => new_nodes,
                }
            }
        };

        let frame = match scope.render_cnt % 2 {
            0 => &mut scope.node_arena_1,
            1 => &mut scope.node_arena_2,
            _ => unreachable!(),
        };

        // set the head of the bump frame
        let alloced = frame.bump.alloc(res);
        frame.node.set(alloced);

        // rebind the lifetime now that its stored internally
        unsafe { std::mem::transmute(alloced) }
    }
}
