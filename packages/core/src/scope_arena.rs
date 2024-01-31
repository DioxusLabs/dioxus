use crate::{
    any_props::{AnyProps, BoxedAnyProps},
    innerlude::{DirtyScope, ScopeState},
    nodes::RenderReturn,
    scope_context::Scope,
    scopes::ScopeId,
    virtual_dom::VirtualDom,
};

impl VirtualDom {
    pub(super) fn new_scope(&mut self, props: BoxedAnyProps, name: &'static str) -> &ScopeState {
        let parent_id = self.runtime.current_scope_id();
        let height = parent_id
            .and_then(|parent_id| self.runtime.get_state(parent_id).map(|f| f.height + 1))
            .unwrap_or(0);
        let entry = self.scopes.vacant_entry();
        let id = ScopeId(entry.key());

        let scope = entry.insert(ScopeState {
            runtime: self.runtime.clone(),
            context_id: id,
            props,
            last_rendered_node: Default::default(),
        });

        self.runtime
            .create_scope(Scope::new(name, id, parent_id, height));

        scope
    }

    pub(crate) fn run_scope(&mut self, scope_id: ScopeId) -> RenderReturn {
        debug_assert!(
            crate::Runtime::current().is_some(),
            "Must be in a dioxus runtime"
        );

        self.runtime.scope_stack.borrow_mut().push(scope_id);
        let scope = &self.scopes[scope_id.0];
        let new_nodes = {
            let context = scope.state();

            context.suspended.set(false);
            context.hook_index.set(0);

            // Run all pre-render hooks
            for pre_run in context.before_render.borrow_mut().iter_mut() {
                pre_run();
            }

            // safety: due to how we traverse the tree, we know that the scope is not currently aliased
            let props: &dyn AnyProps = &*scope.props;

            let span = tracing::trace_span!("render", scope = %scope.state().name);
            span.in_scope(|| props.render())
        };

        let context = scope.state();

        // Run all post-render hooks
        for post_run in context.after_render.borrow_mut().iter_mut() {
            post_run();
        }

        // And move the render generation forward by one
        context.render_count.set(context.render_count.get() + 1);

        // remove this scope from dirty scopes
        self.dirty_scopes.remove(&DirtyScope {
            height: context.height,
            id: context.id,
        });

        if context.suspended.get() {
            if matches!(new_nodes, RenderReturn::Aborted(_)) {
                self.suspended_scopes.insert(context.id);
            }
        } else if !self.suspended_scopes.is_empty() {
            _ = self.suspended_scopes.remove(&context.id);
        }

        self.runtime.scope_stack.borrow_mut().pop();

        new_nodes
    }
}
