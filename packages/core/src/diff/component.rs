use crate::{
    Element,
    diff::{
        context::{DiffContext, DiffFrame, DiffState},
        placement::{DomAnchor, InsertionSite, at_site, insertion_site_for_slot},
        template::DynamicNodeSlot,
    },
    innerlude::{ElementRef, MountId, VComponent, WriteMutations},
    mutations::reborrow_writer,
    nodes::VNode,
    scopes::{LastRenderedNode, ScopeId},
    virtual_dom::VirtualDom,
};

/// Invoke a scope's render driver with the writer erased to
/// `dyn WriteMutations`, checking that the driver leaves the runtime scope
/// stack balanced.
fn drive<R>(
    state: &mut DiffState<'_, '_, '_>,
    f: impl FnOnce(&mut VirtualDom, Option<&mut dyn WriteMutations>) -> R,
) -> R {
    let dom = &mut *state.dom;
    let to = reborrow_writer(&mut state.to);
    #[cfg(debug_assertions)]
    let depth = dom.runtime.scope_stack_depth();
    let result = f(&mut *dom, to);
    #[cfg(debug_assertions)]
    dioxus_debug_assert_eq!(
        depth,
        dom.runtime.scope_stack_depth(),
        "render driver left the runtime scope stack unbalanced"
    );
    result
}

impl VirtualDom {
    pub(crate) fn run_and_diff_scope(
        &mut self,
        to: Option<&mut dyn WriteMutations>,
        scope_id: ScopeId,
    ) {
        self.run_and_diff_scope_with_context(to, scope_id, None);
    }

    pub(crate) fn run_and_diff_scope_with_context(
        &mut self,
        to: Option<&mut dyn WriteMutations>,
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
    pub(crate) fn diff_scope(
        &mut self,
        to: Option<&mut dyn WriteMutations>,
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
            let old = self.scopes[scope.index()]
                .last_rendered_node
                .take()
                .unwrap();

            let old_mount = old.unchecked_mounted_id();

            // If there are suspended scopes, we need to check if the scope is suspended before we diff it
            // If it is suspended, we need to diff it but write the mutations nothing
            // Note: It is important that we still diff the scope even if it is suspended, because the scope may render other child components which may change between renders
            let mut render_to = to.filter(|_| self.scope_should_write_now(scope));
            let mut state =
                DiffState::new_with_context(self, reborrow_writer(&mut render_to), parent_context);
            DiffFrame::new(old_mount, &old, new_real_nodes).diff_into(&mut state);
            if let Some(new_mount) = new_real_nodes.mounted_id() {
                self.replace_mounted_component_root(old_mount, new_mount);
            }

            self.scopes[scope.index()].last_rendered_node = Some(LastRenderedNode::new(new_nodes));

            if render_to.is_some() {
                self.runtime.get_state(scope).mount(&self.runtime);
            }
        })
    }

    /// Create a new [`Scope`](crate::scope_context::Scope) for a component.
    ///
    /// Returns the number of nodes created on the stack
    #[tracing::instrument(skip(self, to), level = "trace", name = "VirtualDom::create_scope")]
    pub(crate) fn create_scope(
        &mut self,
        to: Option<&mut dyn WriteMutations>,
        scope: ScopeId,
        new_nodes: LastRenderedNode,
        parent: Option<ElementRef>,
    ) -> usize {
        self.runtime.clone().with_scope_on_stack(scope, || {
            // If there are suspended scopes, we need to check if the scope is suspended before we diff it
            // If it is suspended, we need to diff it but write the mutations nothing
            // Note: It is important that we still diff the scope even if it is suspended, because the scope may render other child components which may change between renders
            let mut render_to = to.filter(|_| self.scope_should_write_now(scope));

            // Create the node
            let nodes = new_nodes.create(self, parent, reborrow_writer(&mut render_to));

            // Then set the new node as the last rendered node
            self.scopes[scope.index()].last_rendered_node = Some(new_nodes);

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

    pub(crate) fn remove_component_node(
        &mut self,
        to: Option<&mut dyn WriteMutations>,
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
    pub(crate) fn diff_vcomponent(
        &self,
        mount: MountId,
        idx: usize,
        new: &VComponent,
        old: &VComponent,
        scope_id: ScopeId,
        parent: Option<ElementRef>,
        state: &mut DiffState<'_, '_, '_>,
    ) {
        // Replace components whose drivers identify different components
        // (different driver type, or a different body function value)
        if !old.driver.same_component(&*new.driver) {
            return self.replace_vcomponent(mount, idx, new, parent, state);
        }

        // If the props are static, then we try to memoize by setting the new with the old
        // The scope's driver still owns the live props, so there's no need to update anything
        // This also implicitly drops the new props since they're not used
        let scope_driver = state.dom.runtime.get_state(scope_id).render_driver();
        if scope_driver.memoize(new.driver.as_any()) {
            // The scope's driver still owns the live props; memoizing here
            // implicitly drops the new props since they're unused.
            return;
        }

        state.dom.queue_scope(scope_id);
    }

    fn replace_vcomponent(
        &self,
        mount: MountId,
        idx: usize,
        new: &VComponent,
        parent: Option<ElementRef>,
        state: &mut DiffState<'_, '_, '_>,
    ) {
        let scope = state
            .dom
            .unchecked_mounted_dynamic_component_scope(mount, idx);

        // Compute the insertion site BEFORE freeing the scope slot — we need
        // the OLD scope's rendered vnode if it has live DOM. Otherwise we
        // splice into the dynamic slot itself.
        let site = state.dom.scopes[scope.index()]
            .last_rendered_node
            .as_ref()
            .and_then(|n| n.find_first_element(state.dom))
            .map(|id| InsertionSite::AtAnchor(DomAnchor::Before(id)))
            .unwrap_or_else(|| {
                let slot =
                    DynamicNodeSlot::new(&self.template, idx, self.template.dynamic_path(idx));
                insertion_site_for_slot(mount, slot, &[], state.dom, state.context())
            });

        // Free the scope slot so `create_component_node` allocates a new scope.
        state.dom.clear_mounted_dynamic_component_scope(mount, idx);

        {
            let runtime = state.dom.runtime.clone();
            let dom = &mut *state.dom;
            let to = reborrow_writer(&mut state.to);
            at_site(site, to, runtime, |to| {
                let mut state = DiffState::new(dom, to);
                self.create_component_node(mount, idx, new, parent, &mut state)
            });
        }
        state
            .dom
            .remove_component_node(reborrow_writer(&mut state.to), true, scope);
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
        state: &mut DiffState<'_, '_, '_>,
    ) -> usize {
        let mut scope_id = state.dom.mounted_dynamic_component_scope(mount, idx);
        let new = scope_id.is_none();

        // If the scope id is missing, we need to load up a new scope for this
        // vcomponent. If it's already mounted, then we can just use that.
        if new {
            // The scope adopts a duplicate of the vnode's driver so the live
            // scope never aliases props with a vnode (a cached rsx element
            // hands out the same driver instance every render).
            let new_scope_id = state
                .dom
                .new_scope(component.name, component.driver.duplicate())
                .state()
                .id;
            scope_id = Some(new_scope_id);

            // Store the scope id for the next render
            state
                .dom
                .set_mounted_dynamic_component_scope(mount, idx, new_scope_id);
        }

        let scope_id = scope_id.expect("component scope should be mounted");
        let driver = state.dom.runtime.get_state(scope_id).render_driver();
        let nodes = drive(state, |dom, to| {
            driver.create(dom, scope_id, new, parent, to)
        });
        let root = state
            .dom
            .get_scope(scope_id)
            .and_then(|scope| scope.try_root_node())
            .and_then(VNode::mounted_id);
        state
            .dom
            .set_mounted_dynamic_component_root(mount, idx, root);
        nodes
    }
}
