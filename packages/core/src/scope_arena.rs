use crate::innerlude::{
    throw_error, try_consume_context, RenderError, RenderReturn, ScopeOrder, SuspenseContext,
};
use crate::Element;
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
        });

        self.runtime
            .create_scope(Scope::new(name, id, parent_id, height));
        tracing::trace!("created scope {id:?} with parent {parent_id:?}");

        scope
    }

    /// Run a scope and return the rendered nodes. This will not modify the DOM or update the last rendered node of the scope.
    pub(crate) fn run_scope(&mut self, scope_id: ScopeId) -> RenderReturn {
        debug_assert!(
            crate::Runtime::current().is_some(),
            "Must be in a dioxus runtime"
        );
        self.runtime.scope_stack.borrow_mut().push(scope_id);

        let scope = &self.scopes[scope_id.0];
        let output = {
            let scope_state = scope.state();

            scope_state.hook_index.set(0);

            // Run all pre-render hooks
            for pre_run in scope_state.before_render.borrow_mut().iter_mut() {
                pre_run();
            }

            let props: &dyn AnyProps = &*scope.props;

            let span = tracing::trace_span!("render", scope = %scope.state().name);
            span.in_scope(|| {
                let render_return = props.render();
                self.handle_element_return(&render_return.node, scope_id, &scope.state());
                render_return
            })
        };

        let scope_state = scope.state();

        // Run all post-render hooks
        for post_run in scope_state.after_render.borrow_mut().iter_mut() {
            post_run();
        }

        // remove this scope from dirty scopes
        self.dirty_scopes
            .remove(&ScopeOrder::new(scope_state.height, scope_id));

        self.runtime.scope_stack.borrow_mut().pop();

        output
    }

    /// Insert any errors, or suspended tasks from an element return into the runtime
    fn handle_element_return(&self, node: &Element, scope_id: ScopeId, scope_state: &Scope) {
        match &node {
            Err(RenderError::Aborted(e)) => {
                tracing::error!(
                    "Error while rendering component `{}`: {e:?}",
                    scope_state.name
                );
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
                    .get(task.id)
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
                scope_state
                    .render_count
                    .set(scope_state.render_count.get() + 1);
            }
        }
    }
}
