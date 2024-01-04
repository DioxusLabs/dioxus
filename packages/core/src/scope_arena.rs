use crate::{
    any_props::AnyProps,
    bump_frame::BumpFrame,
    innerlude::DirtyScope,
    nodes::RenderReturn,
    scope_context::ScopeContext,
    scopes::{ScopeId, ScopeState},
    virtual_dom::VirtualDom,
};

impl VirtualDom {
    pub(super) fn new_scope(
        &mut self,
        props: Box<dyn AnyProps<'static>>,
        name: &'static str,
    ) -> &ScopeState {
        let parent_id = self.runtime.current_scope_id();
        let height = parent_id
            .and_then(|parent_id| self.get_scope(parent_id).map(|f| f.context().height + 1))
            .unwrap_or(0);
        let entry = self.scopes.vacant_entry();
        let id = ScopeId(entry.key());

        let scope = entry.insert(Box::new(ScopeState {
            runtime: self.runtime.clone(),
            context_id: id,

            props: Some(props),

            node_arena_1: BumpFrame::new(0),
            node_arena_2: BumpFrame::new(0),

            render_cnt: Default::default(),
            hooks: Default::default(),
            hook_idx: Default::default(),

            borrowed_props: Default::default(),
            attributes_to_drop_before_render: Default::default(),
            element_refs_to_drop: Default::default(),
        }));

        let context =
            ScopeContext::new(name, id, parent_id, height, self.runtime.scheduler.clone());
        self.runtime.create_context_at(id, context);

        scope
    }

    pub(crate) fn run_scope(&mut self, scope_id: ScopeId) -> &RenderReturn {
        self.runtime.scope_stack.borrow_mut().push(scope_id);
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        // Remove all the outdated listeners
        self.ensure_drop_safety(scope_id);

        let new_nodes = unsafe {
            let scope = &self.scopes[scope_id.0];
            scope.previous_frame().reset();

            scope.context().suspended.set(false);

            scope.hook_idx.set(0);

            // safety: due to how we traverse the tree, we know that the scope is not currently aliased
            let props: &dyn AnyProps = scope.props.as_ref().unwrap().as_ref();
            let props: &dyn AnyProps = std::mem::transmute(props);

            let _span = tracing::trace_span!("render", scope = %scope.context().name);
            props.render(scope).extend_lifetime()
        };

        let scope = &self.scopes[scope_id.0];

        // We write on top of the previous frame and then make it the current by pushing the generation forward
        let frame = scope.previous_frame();

        // set the new head of the bump frame
        let allocated = &*frame.bump().alloc(new_nodes);
        frame.node.set(allocated);

        // And move the render generation forward by one
        scope.render_cnt.set(scope.render_cnt.get() + 1);

        let context = scope.context();
        // remove this scope from dirty scopes
        self.dirty_scopes.remove(&DirtyScope {
            height: context.height,
            id: context.id,
        });

        if context.suspended.get() {
            if matches!(allocated, RenderReturn::Aborted(_)) {
                self.suspended_scopes.insert(context.id);
            }
        } else if !self.suspended_scopes.is_empty() {
            _ = self.suspended_scopes.remove(&context.id);
        }

        // rebind the lifetime now that its stored internally
        let result = unsafe { allocated.extend_lifetime_ref() };

        self.runtime.scope_stack.borrow_mut().pop();

        result
    }
}
