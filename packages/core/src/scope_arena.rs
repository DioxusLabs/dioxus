use crate::{
    any_props::AnyProps,
    bump_frame::BumpFrame,
    innerlude::{DirtyScope, ScopeId, ScopeState},
    virtual_dom::VirtualDom,
    Element,
};
use std::mem;

impl VirtualDom {
    pub(super) fn new_scope(
        &mut self,
        props: Box<dyn AnyProps<'static>>,
        name: &'static str,
    ) -> &ScopeState {
        let parent = self.acquire_current_scope_raw();
        let height = unsafe { parent.map(|f| (*f).height + 1).unwrap_or(0) };
        let entry = self.scopes.vacant_entry();
        let id = entry.key();

        entry.insert(ScopeState {
            parent,
            id,
            height,
            name,
            props: Some(props),
            tasks: self.scheduler.clone(),
            node_arena_1: BumpFrame::new(0),
            node_arena_2: BumpFrame::new(0),
            spawned_tasks: Default::default(),
            render_cnt: Default::default(),
            hooks: Default::default(),
            shared_contexts: Default::default(),
            borrowed_props: Default::default(),
            attributes_to_drop: Default::default(),
        })
    }

    fn acquire_current_scope_raw(&self) -> Option<*const ScopeState> {
        let id = self.scope_stack.last().copied()?;
        let scope = self.scopes.get(id)?;
        Some(scope)
    }

    pub(crate) fn run_scope(&mut self, scope_id: ScopeId) -> &Element {
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        // Remove all the outdated listeners
        self.ensure_drop_safety(scope_id);

        let new_nodes = unsafe {
            self.scopes[scope_id].previous_frame().bump_mut().reset();

            let scope = &self.scopes[scope_id];

            scope.hooks.new_render();

            // safety: due to how we traverse the tree, we know that the scope is not currently aliased
            let props: &dyn AnyProps = scope.props.as_ref().unwrap().as_ref();
            let props: &dyn AnyProps = mem::transmute(props);

            todo!()
            // props.render(scope)
        };

        let scope = &self.scopes[scope_id];

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

        todo!()
        // rebind the lifetime now that its stored internally
        // unsafe { allocated }
    }
}
