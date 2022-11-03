use crate::{
    any_props::AnyProps,
    arena::ElementId,
    bump_frame::BumpFrame,
    nodes::VNode,
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
            tasks: self.pending_futures.clone(),
            node_arena_1: BumpFrame::new(50),
            node_arena_2: BumpFrame::new(50),
            render_cnt: Default::default(),
            hook_arena: Default::default(),
            hook_vals: Default::default(),
            hook_idx: Default::default(),
            shared_contexts: Default::default(),
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

    pub fn run_scope(&mut self, id: ScopeId) -> &VNode {
        let scope = &mut self.scopes[id.0];
        scope.hook_idx.set(0);

        let res = {
            let props = unsafe { &mut *scope.props };
            let props: &mut dyn AnyProps = unsafe { std::mem::transmute(props) };
            let res: VNode = props.render(scope).unwrap();
            let res: VNode<'static> = unsafe { std::mem::transmute(res) };
            res
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
