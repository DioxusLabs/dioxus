use crate::{
    Element,
    diff::{
        anchor::{Anchor, anchor_for_slot, at_anchor},
        context::{DiffContext, DiffFrame, DiffState},
    },
    innerlude::{ElementRef, MountId, ScopeOrder, VComponent, WriteMutations},
    nodes::VNode,
    scopes::{LastRenderedNode, ScopeId},
    virtual_dom::VirtualDom,
};

/// Invoke a scope's render driver with the writer erased to
/// `dyn WriteMutations`, checking that the driver leaves the runtime scope
/// stack balanced.
fn drive<M: WriteMutations, R>(
    state: &mut DiffState<'_, M>,
    f: impl FnOnce(&mut VirtualDom, Option<&mut dyn WriteMutations>) -> R,
) -> R {
    let dom = &mut *state.dom;
    let to = state
        .to
        .as_deref_mut()
        .map(|m| m as &mut dyn WriteMutations);
    #[cfg(debug_assertions)]
    let depth = dom.runtime.scope_stack_depth();
    let result = f(&mut *dom, to);
    #[cfg(debug_assertions)]
    debug_assert_eq!(
        depth,
        dom.runtime.scope_stack_depth(),
        "render driver left the runtime scope stack unbalanced"
    );
    result
}

impl VirtualDom {
    pub(crate) fn run_and_diff_scope<M: WriteMutations>(
        &mut self,
        to: Option<&mut M>,
        scope_id: ScopeId,
    ) {
        self.run_and_diff_scope_with_context(to, scope_id, None);
    }

    pub(crate) fn run_and_diff_scope_with_context<M: WriteMutations>(
        &mut self,
        to: Option<&mut M>,
        scope_id: ScopeId,
        parent_context: Option<DiffContext<'_>>,
    ) {
        let mut state = DiffState::new_with_context(self, to, parent_context);
        let context = state.context();
        let driver = state.dom.runtime.get_state(scope_id).render_driver();
        drive(&mut state, |dom, to| {
            driver.diff(dom, scope_id, context, to)
        })
    }

    #[tracing::instrument(skip(self, to), level = "trace", name = "VirtualDom::diff_scope")]
    pub(crate) fn diff_scope<M: WriteMutations>(
        &mut self,
        to: Option<&mut M>,
        scope: ScopeId,
        new_nodes: Element,
        parent_context: Option<DiffContext<'_>>,
    ) {
        self.runtime.clone().with_scope_on_stack(scope, || {
            // We don't diff the nodes if the scope is suspended or has an error
            let Ok(new_real_nodes) = &new_nodes else {
                return;
            };
            // Load the old and new rendered nodes
            let old = self.scopes[scope.0].last_rendered_node.take().unwrap();

            if !old.mount.get().mounted() {
                // The previous output was never materialized (it is awaiting
                // the foreground re-create pass of a resolving suspense
                // boundary). There is nothing to diff against: adopt the body
                // output; the create pass mounts it.
                self.scopes[scope.0].last_rendered_node = Some(LastRenderedNode::new(new_nodes));
                return;
            }

            // If there are suspended scopes, we need to check if the scope is suspended before we diff it
            // If it is suspended, we need to diff it but write the mutations nothing
            // Note: It is important that we still diff the scope even if it is suspended, because the scope may render other child components which may change between renders
            let target_id = self.runtime.get_state(scope).target_id();
            let mut render_to = to
                .filter(|_| self.scope_should_write_now(scope))
                .filter(|_| self.render_target_should_write(target_id));
            let mut state =
                DiffState::new_with_context(self, render_to.as_deref_mut(), parent_context);
            DiffFrame::new(old.mount.get(), &old, new_real_nodes).diff_into(&mut state);

            self.scopes[scope.0].last_rendered_node = Some(LastRenderedNode::new(new_nodes));

            if render_to.is_some() {
                self.runtime.get_state(scope).mount(&self.runtime);
            }
        })
    }

    /// Create a new [`Scope`](crate::scope_context::Scope) for a component.
    ///
    /// Returns the number of nodes created on the stack
    #[tracing::instrument(skip(self, to), level = "trace", name = "VirtualDom::create_scope")]
    pub(crate) fn create_scope<M: WriteMutations>(
        &mut self,
        to: Option<&mut M>,
        scope: ScopeId,
        new_nodes: LastRenderedNode,
        parent: Option<ElementRef>,
    ) -> usize {
        self.runtime.clone().with_scope_on_stack(scope, || {
            // If there are suspended scopes, we need to check if the scope is suspended before we diff it
            // If it is suspended, we need to diff it but write the mutations nothing
            // Note: It is important that we still diff the scope even if it is suspended, because the scope may render other child components which may change between renders
            let target_id = self.runtime.get_state(scope).target_id();
            let mut render_to = to
                .filter(|_| self.scope_should_write_now(scope))
                .filter(|_| self.render_target_should_write(target_id));

            // Create the node
            let nodes = new_nodes.create(self, parent, render_to.as_deref_mut());

            // Then set the new node as the last rendered node
            self.scopes[scope.0].last_rendered_node = Some(new_nodes);

            if render_to.is_some() {
                self.runtime.get_state(scope).mount(&self.runtime);
            }

            nodes
        })
    }

    pub(crate) fn scope_should_write_now(&self, scope: ScopeId) -> bool {
        self.runtime.scope_should_render(scope)
            || self
                .runtime
                .current_suspense_location()
                .is_some_and(|location| location.should_write())
    }

    pub(crate) fn remove_component_node<M: WriteMutations>(
        &mut self,
        to: Option<&mut M>,
        destroy_component_state: bool,
        scope_id: ScopeId,
    ) {
        let mut state = DiffState::new(self, to);
        let driver = state.dom.runtime.get_state(scope_id).render_driver();
        drive(&mut state, |dom, to| {
            driver.remove(dom, scope_id, to, destroy_component_state)
        })
    }
}

impl VNode {
    pub(crate) fn diff_vcomponent<M: WriteMutations>(
        &self,
        mount: MountId,
        idx: usize,
        new: &VComponent,
        old: &VComponent,
        scope_id: ScopeId,
        parent: Option<ElementRef>,
        state: &mut DiffState<'_, M>,
    ) {
        // Replace components whose drivers identify different components
        // (different driver type, or a different body function value)
        if !old.driver.same_component(&*new.driver) {
            return self.replace_vcomponent(mount, idx, new, parent, state);
        }

        // If the props are static, then we try to memoize by setting the new with the old
        // The scope's driver still owns the live props, so there's no need to update anything
        // This also implicitly drops the new props since they're not used
        let height = state.dom.runtime.get_state(scope_id).height;

        let scope_driver = state.dom.runtime.get_state(scope_id).render_driver();
        if scope_driver.memoize(new.driver.as_any()) {
            // The scope's driver still owns the live props; memoizing here
            // implicitly drops the new props since they're unused.
            return;
        }

        state.dom.queue_scope(ScopeOrder::new(height, scope_id));
    }

    fn replace_vcomponent<M: WriteMutations>(
        &self,
        mount: MountId,
        idx: usize,
        new: &VComponent,
        parent: Option<ElementRef>,
        state: &mut DiffState<'_, M>,
    ) {
        let scope = ScopeId(state.dom.get_mounted_dyn_node(mount, idx));

        // Compute the anchor BEFORE freeing the scope slot — we need the OLD
        // scope's rendered vnode to anchor against. If the OLD scope rendered
        // DOM, that DOM is our insertion neighbor; otherwise we splice into
        // the dynamic slot itself.
        let slot_path: &[u8] = parent.as_ref().map_or(&[], |p| p.path.path);
        let anchor = state.dom.scopes[scope.0]
            .last_rendered_node
            .as_ref()
            .and_then(|n| n.find_first_element(state.dom))
            .map(Anchor::Before)
            .unwrap_or_else(|| anchor_for_slot(mount, slot_path, &[], state.dom, state.context()));

        // Free the scope slot so `create_component_node` allocates a new scope.
        state
            .dom
            .set_mounted_dyn_node(mount, idx, ScopeId::PLACEHOLDER.0);

        {
            let dom = &mut *state.dom;
            let to = state.to.as_deref_mut();
            at_anchor(anchor, to, |to| {
                let mut state = DiffState::new(dom, to);
                self.create_component_node(mount, idx, new, parent, &mut state)
            });
        }
        state
            .dom
            .remove_component_node(state.to.as_deref_mut(), true, scope);
    }

    /// Create a new component (if it doesn't already exist) node and then mount the [`crate::ScopeState`] for a component
    ///
    /// Returns the number of nodes created on the stack
    pub(super) fn create_component_node(
        &self,
        mount: MountId,
        idx: usize,
        component: &VComponent,
        parent: Option<ElementRef>,
        state: &mut DiffState<'_, impl WriteMutations>,
    ) -> usize {
        let mut scope_id = ScopeId(state.dom.get_mounted_dyn_node(mount, idx));
        let new = scope_id.is_placeholder();

        // If the scopeid is a placeholder, we need to load up a new scope for this vcomponent. If it's already mounted, then we can just use that
        if new {
            // The scope adopts a duplicate of the vnode's driver so the live
            // scope never aliases props with a vnode (a cached rsx element
            // hands out the same driver instance every render).
            scope_id = state
                .dom
                .new_scope(component.name, component.driver.duplicate())
                .state()
                .id;

            // Store the scope id for the next render
            state.dom.set_mounted_dyn_node(mount, idx, scope_id.0);
        }

        let driver = state.dom.runtime.get_state(scope_id).render_driver();
        drive(state, |dom, to| {
            driver.create(dom, scope_id, new, parent, to)
        })
    }
}
