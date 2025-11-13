use crate::{
    any_props::{AnyProps, BoxedAnyProps},
    innerlude::{RenderError, ScopeOrder, ScopeState},
    scope_context::{Scope, SuspenseLocation},
    scopes::ScopeId,
    virtual_dom::VirtualDom,
    Element, ReactiveContext,
};

impl VirtualDom {
    pub(super) fn new_scope(
        &mut self,
        props: BoxedAnyProps,
        name: &'static str,
    ) -> &mut ScopeState {
        let parent_id = self.runtime.try_current_scope_id();
        let height = match parent_id.and_then(|id| self.runtime.try_get_state(id)) {
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

        scope
    }

    /// Run a scope and return the rendered nodes. This will not modify the DOM or update the last rendered node of the scope.
    #[tracing::instrument(skip(self), level = "trace", name = "VirtualDom::run_scope")]
    #[track_caller]
    pub(crate) fn run_scope(&mut self, scope_id: ScopeId) -> Element {
        // Ensure we are currently inside a `Runtime`.
        crate::Runtime::current();

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
                        let render_return = props.render();
                        // After the component is run, we need to do a deep clone of the VNode. This
                        // breaks any references to mounted parts of the VNode from the component.
                        // Without this, the component could store a mounted version of the VNode
                        // which causes a lot of issues for diffing because we expect only the old
                        // or new node to be mounted.
                        //
                        // For example, the dog app example returns rsx from a resource. Every time
                        // the component runs, it returns a clone of the last rsx that was returned from
                        // that resource. If we don't deep clone the VNode and the resource changes, then
                        // we could end up diffing two different versions of the same mounted node
                        let mut render_return = match render_return {
                            Ok(node) => Ok(node.deep_clone()),
                            Err(RenderError::Error(err)) => Err(RenderError::Error(err.clone())),
                            Err(RenderError::Suspended(fut)) => {
                                Err(RenderError::Suspended(fut.deep_clone()))
                            }
                        };

                        self.handle_element_return(&mut render_return, &scope.state());
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
    fn handle_element_return(&self, node: &mut Element, scope: &Scope) {
        match node {
            Err(RenderError::Error(e)) => {
                tracing::error!("Error while rendering component `{}`: {e}", scope.name);
                self.runtime.throw_error(scope.id, e.clone());
            }
            Err(RenderError::Suspended(e)) => {
                let task = e.task();
                // Insert the task into the nearest suspense boundary if it exists
                let boundary = scope.suspense_location();
                let already_suspended = self
                    .runtime
                    .tasks
                    .borrow()
                    .get(task.id)
                    .expect("Suspended on a task that no longer exists")
                    .suspend(boundary.clone());
                if !already_suspended {
                    tracing::trace!("Suspending {:?} on {:?}", scope.id, task);
                    // Add this task to the suspended tasks list of the boundary
                    if let SuspenseLocation::UnderSuspense(boundary) = &boundary {
                        boundary.add_suspended_task(e.clone());
                    }
                    self.runtime
                        .suspended_tasks
                        .set(self.runtime.suspended_tasks.get() + 1);
                }
            }
            Ok(_) => {
                // If the render was successful, we can move the render generation forward by one
                scope.render_count.set(scope.render_count.get() + 1);
            }
        }
    }
}
