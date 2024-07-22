use crate::innerlude::{throw_error, RenderError, RenderReturn, ScopeOrder};
use crate::prelude::ReactiveContext;
use crate::scope_context::SuspenseLocation;
use crate::{
    any_props::{AnyProps, BoxedAnyProps},
    innerlude::ScopeState,
    scope_context::Scope,
    scopes::ScopeId,
    virtual_dom::VirtualDom,
};
use crate::{Element, VNode};

impl VirtualDom {
    pub(super) fn new_scope(
        &mut self,
        props: BoxedAnyProps,
        name: &'static str,
    ) -> &mut ScopeState {
        let parent_id = self.runtime.current_scope_id().ok();
        let height = match parent_id.and_then(|id| self.runtime.get_state(id)) {
            Some(parent) => parent.height() + 1,
            None => 0,
        };
        let suspense_boundary = self
            .runtime
            .current_suspense_location()
            .unwrap_or(SuspenseLocation::NotSuspended);
        let entry = self.scopes.vacant_entry();
        let id = ScopeId(entry.key());

        let scope_runtime = Scope::new(name, id, parent_id, height, suspense_boundary);
        let reactive_context = ReactiveContext::new_for_scope(&scope_runtime, &self.runtime);

        let scope = entry.insert(ScopeState {
            runtime: self.runtime.clone(),
            context_id: id,
            props,
            last_rendered_node: Default::default(),
            reactive_context,
        });

        self.runtime.create_scope(scope_runtime);
        tracing::trace!("created scope {id:?} with parent {parent_id:?}");

        scope
    }

    /// Run a scope and return the rendered nodes. This will not modify the DOM or update the last rendered node of the scope.
    #[tracing::instrument(skip(self), level = "trace", name = "VirtualDom::run_scope")]
    #[track_caller]
    pub(crate) fn run_scope(&mut self, scope_id: ScopeId) -> RenderReturn {
        // Ensure we are currently inside a `Runtime`.
        crate::Runtime::current().unwrap();

        self.runtime.clone().with_scope_on_stack(scope_id, || {
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
                    scope.reactive_context.reset_and_run_in(|| {
                        let mut render_return = props.render();
                        self.handle_element_return(
                            &mut render_return.node,
                            scope_id,
                            &scope.state(),
                        );
                        render_return
                    })
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
            output
        })
    }

    /// Insert any errors, or suspended tasks from an element return into the runtime
    fn handle_element_return(&self, node: &mut Element, scope_id: ScopeId, scope_state: &Scope) {
        match node {
            Err(RenderError::Aborted(e)) => {
                tracing::error!(
                    "Error while rendering component `{}`:\n{e}",
                    scope_state.name
                );
                throw_error(e.clone_mounted());
                e.render = VNode::placeholder();
            }
            Err(RenderError::Suspended(e)) => {
                let task = e.task();
                // Insert the task into the nearest suspense boundary if it exists
                let boundary = self
                    .runtime
                    .get_state(scope_id)
                    .unwrap()
                    .suspense_location();
                let already_suspended = self
                    .runtime
                    .tasks
                    .borrow()
                    .get(task.id)
                    .expect("Suspended on a task that no longer exists")
                    .suspend(boundary.clone());
                if !already_suspended {
                    tracing::trace!("Suspending {:?} on {:?}", scope_id, task);
                    // Add this task to the suspended tasks list of the boundary
                    if let SuspenseLocation::UnderSuspense(boundary) = &boundary {
                        boundary.add_suspended_task(e.clone());
                    }
                    self.runtime
                        .suspended_tasks
                        .set(self.runtime.suspended_tasks.get() + 1);
                }
                e.placeholder = VNode::placeholder();
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
