use std::rc::Rc;

use crate::{
    Element, ReactiveContext,
    innerlude::{RenderError, ScopeState, SuspendedTaskRegistration},
    render_driver::RenderDriver,
    scope_context::{Scope, SuspenseLocation},
    scopes::ScopeId,
    virtual_dom::VirtualDom,
};

impl VirtualDom {
    /// Create a scope rendering into the current scope's render target (the
    /// root target when no scope is active). `driver` owns the scope's
    /// rendering lifecycle and props; portal drivers retarget the scope
    /// during their first create.
    pub(super) fn new_scope(
        &mut self,
        name: &'static str,
        driver: Rc<dyn RenderDriver>,
    ) -> &mut ScopeState {
        let target_id = self.runtime.current_render_target_id();
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
        let id = ScopeId::new(entry.key());

        let scope_runtime = Scope::new(
            name,
            id,
            parent_id,
            target_id,
            height,
            suspense_boundary,
            driver,
        );
        let reactive_context = ReactiveContext::new_for_scope(&scope_runtime, &self.runtime);

        let scope = entry.insert(ScopeState {
            runtime: self.runtime.clone(),
            context_id: id,
            height,
            last_rendered_node: Default::default(),
            reactive_context,
        });

        self.runtime.create_scope(scope_runtime);

        scope
    }

    /// Run a scope's body via `render` and return the rendered nodes. This
    /// will not modify the DOM or update the last rendered node of the scope.
    #[tracing::instrument(skip(self, render), level = "trace", name = "VirtualDom::run_scope")]
    #[track_caller]
    pub(crate) fn run_scope_with(
        &mut self,
        scope_id: ScopeId,
        render: impl FnOnce() -> Element,
    ) -> Element {
        // Ensure we are currently inside a `Runtime`.
        crate::Runtime::current();

        self.runtime.clone().with_scope_on_stack(scope_id, || {
            let scope = &self.scopes[scope_id.index()];
            let output = {
                let scope_state = scope.state();

                scope_state.hook_index.set(0);

                // Run all pre-render hooks
                for pre_run in scope_state.before_render.borrow_mut().iter_mut() {
                    pre_run();
                }

                let span = tracing::trace_span!("render", scope = %scope.state().name);
                span.in_scope(|| {
                    scope.reactive_context.reset_and_run_in(|| {
                        let mut render_return = render();

                        self.handle_element_return(&mut render_return, &scope.state());
                        render_return
                    })
                })
            };

            {
                let scope_state = scope.state();

                // Run all post-render hooks
                for post_run in scope_state.after_render.borrow_mut().iter_mut() {
                    post_run();
                }
            }

            // remove this scope from dirty scopes
            self.mark_clean(scope_id);
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
                let registration = self
                    .runtime
                    .tasks
                    .borrow()
                    .get(task.id)
                    .expect("Suspended on a task that no longer exists")
                    .suspend(boundary.clone());
                match registration {
                    SuspendedTaskRegistration::New | SuspendedTaskRegistration::Moved { .. } => {
                        tracing::trace!("Suspending {:?} on {:?}", scope.id, task);

                        if let SuspendedTaskRegistration::Moved { old_boundary } = &registration
                            && let Some(old_boundary) = old_boundary.suspense_context()
                        {
                            old_boundary.remove_suspended_task(task);
                        }

                        // Every user-rendered scope sits inside the implicit
                        // `SuspenseBoundary` from `RootScopeWrapper`, so a
                        // suspended scope's location always carries a boundary
                        // context (`UnderSuspense` or `InSuspensePlaceholder`).
                        boundary
                            .suspense_context()
                            .expect("suspended scope must have a SuspenseContext")
                            .add_suspended_task(e.clone());

                        if matches!(registration, SuspendedTaskRegistration::New) {
                            self.runtime
                                .suspended_tasks
                                .set(self.runtime.suspended_tasks.get() + 1);
                        }
                    }
                    SuspendedTaskRegistration::Unchanged => {}
                }
            }
            Ok(_) => {
                // If the render was successful, we can move the render generation forward by one
                scope.render_count.set(scope.render_count.get() + 1);
            }
        }
    }
}
