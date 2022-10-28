use slab::Slab;

use crate::{
    any_props::AnyProps,
    arena::ElementId,
    bump_frame::BumpFrame,
    nodes::VTemplate,
    scopes::{ComponentPtr, ScopeId, ScopeState},
    VirtualDom,
};

impl VirtualDom {
    pub fn new_scope(
        &mut self,
        fn_ptr: ComponentPtr,
        parent: Option<*mut ScopeState>,
        container: ElementId,
        props: Box<dyn AnyProps>,
    ) -> ScopeId {
        let entry = self.scopes.vacant_entry();
        let our_arena_idx = entry.key();
        let height = unsafe { parent.map(|f| (*f).height).unwrap_or(0) + 1 };

        entry.insert(ScopeState {
            parent,
            container,
            our_arena_idx,
            height,
            fn_ptr,
            props,
            node_arena_1: BumpFrame::new(50),
            node_arena_2: BumpFrame::new(50),
            render_cnt: Default::default(),
            hook_arena: Default::default(),
            hook_vals: Default::default(),
            hook_idx: Default::default(),
        });

        our_arena_idx
    }

    pub fn run_scope<'a>(&'a mut self, id: ScopeId) -> &'a VTemplate<'a> {
        let scope = &mut self.scopes[id];
        scope.hook_idx.set(0);

        let res = scope.props.render(scope).unwrap();
        let res: VTemplate<'static> = unsafe { std::mem::transmute(res) };

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
