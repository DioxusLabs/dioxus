use crate::innerlude::{
    throw_error, try_consume_context, RenderError, RenderReturn, ScopeOrder, SuspenseContext,
};
use crate::{
    any_props::{AnyProps, BoxedAnyProps},
    innerlude::ScopeState,
    scope_context::Scope,
    scopes::ScopeId,
    virtual_dom::VirtualDom,
};

impl VirtualDom {
    pub(super) fn new_scope(
        &mut self,
        props: BoxedAnyProps,
        name: &'static str,
    ) -> &mut ScopeState {
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
            last_mounted_node: Default::default(),
        });

        self.runtime
            .create_scope(Scope::new(name, id, parent_id, height));

        scope
    }

    /// Run a scope and return the rendered nodes. This will not modify the DOM or update the last rendered node of the scope.
    pub(crate) fn run_scope(&mut self, scope_id: ScopeId) -> RenderReturn {
        tracing::info!("Running scope {scope_id:?}");

        debug_assert!(
            crate::Runtime::current().is_some(),
            "Must be in a dioxus runtime"
        );

        self.runtime.scope_stack.borrow_mut().push(scope_id);
        let scope = &self.scopes[scope_id.0];
        let new_nodes = {
            let context = scope.state();

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

        // remove this scope from dirty scopes
        self.dirty_scopes
            .remove(&ScopeOrder::new(context.height, scope_id));

        match &new_nodes.node {
            Err(RenderError::Aborted(e)) => {
                throw_error(e.clone());
            }
            Err(RenderError::Suspended(e)) => {
                let task = e.task();
                // Insert the task into the nearest suspense boundary if it exists
                let boundary = try_consume_context::<SuspenseContext>();
                let already_suspended = self
                    .runtime
                    .tasks
                    .borrow()
                    .get(task.0)
                    .unwrap()
                    .suspend(boundary.clone());
                if !already_suspended {
                    tracing::trace!("Suspending {:?} on {:?}", scope_id, task);
                    // Add this task to the suspended tasks list of the boundary
                    if let Some(boundary) = &boundary {
                        boundary.add_suspended_task(e.clone());
                    }
                    self.runtime
                        .suspended_tasks
                        .set(self.runtime.suspended_tasks.get() + 1);
                }
            }
            Ok(_) => {
                // If the render was successful, we can move the render generation forward by one
                context.render_count.set(context.render_count.get() + 1);
            }
        }

        self.runtime.scope_stack.borrow_mut().pop();

        new_nodes
    }
}
