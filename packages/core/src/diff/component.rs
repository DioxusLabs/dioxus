use crate::{
    innerlude::{ElementRef, MountId, ScopeOrder, VComponent, WriteMutations},
    nodes::VNode,
    scopes::{LastRenderedNode, ScopeId},
    virtual_dom::VirtualDom,
};

impl VirtualDom {
    pub(crate) fn run_and_diff_scope<M: WriteMutations>(
        &mut self,
        to: Option<&mut M>,
        scope_id: ScopeId,
    ) {
        let driver = self.runtime.get_state(scope_id).render_driver();
        driver.diff(self, scope_id, to);
    }

    #[tracing::instrument(skip(self, to), level = "trace", name = "VirtualDom::diff_scope")]
    pub(crate) fn diff_scope<M: WriteMutations>(
        &mut self,
        to: Option<&mut M>,
        scope: ScopeId,
        new_nodes: crate::Element,
    ) {
        self.runtime.clone().with_scope_on_stack(scope, || {
            // We don't diff the nodes if the scope is suspended or has an error
            let Ok(new_real_nodes) = &new_nodes else {
                return;
            };
            let scope_state = &mut self.scopes[scope.0];
            // Load the old and new rendered nodes
            let old = scope_state.last_rendered_node.take().unwrap();

            // If there are suspended scopes, we need to check if the scope is suspended before we diff it
            // If it is suspended, we need to diff it but write the mutations nothing
            // Note: It is important that we still diff the scope even if it is suspended, because the scope may render other child components which may change between renders
            let mut render_to = to.filter(|_| self.runtime.scope_should_render(scope));
            old.diff_node(new_real_nodes, self, render_to.as_deref_mut());

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
            let mut render_to = to.filter(|_| self.runtime.scope_should_render(scope));

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

    pub(crate) fn remove_component_node<M: WriteMutations>(
        &mut self,
        to: Option<&mut M>,
        destroy_component_state: bool,
        scope_id: ScopeId,
        replace_with: Option<usize>,
    ) {
        let driver = self.runtime.get_state(scope_id).render_driver();
        driver.remove(self, scope_id, to, destroy_component_state, replace_with);
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
        dom: &mut VirtualDom,
        to: Option<&mut impl WriteMutations>,
    ) {
        // Replace components whose drivers identify different components
        // (different driver type, or a different body function value)
        if !old.driver.same_component(&new.driver) {
            return self.replace_vcomponent(mount, idx, new, parent, dom, to);
        }

        // If the props are static, then we try to memoize by setting the new with the old
        // The scope's driver still owns the live props, so there's no need to update anything
        // This also implicitly drops the new props since they're not used
        let scope_driver = dom.runtime.get_state(scope_id).render_driver();
        if scope_driver.memoize(&new.driver) {
            return;
        }

        // Now diff the scope
        dom.run_and_diff_scope(to, scope_id);

        let height = dom.runtime.get_state(scope_id).height;
        dom.dirty_scopes.remove(&ScopeOrder::new(height, scope_id));
    }

    fn replace_vcomponent(
        &self,
        mount: MountId,
        idx: usize,
        new: &VComponent,
        parent: Option<ElementRef>,
        dom: &mut VirtualDom,
        mut to: Option<&mut impl WriteMutations>,
    ) {
        let scope = ScopeId(dom.get_mounted_dyn_node(mount, idx));

        // Remove the scope id from the mount
        dom.set_mounted_dyn_node(mount, idx, ScopeId::PLACEHOLDER.0);
        let m = self.create_component_node(mount, idx, new, parent, dom, to.as_deref_mut());

        // Instead of *just* removing it, we can use the replace mutation
        dom.remove_component_node(to, true, scope, Some(m));
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
        dom: &mut VirtualDom,
        to: Option<&mut impl WriteMutations>,
    ) -> usize {
        let mut scope_id = ScopeId(dom.get_mounted_dyn_node(mount, idx));
        let new = scope_id.is_placeholder();

        // If the scope id is a placeholder, we need to load up a new scope for this
        // vcomponent. If it's already mounted, then we can just use that.
        if new {
            scope_id = dom
                .new_scope(component.name, component.driver.duplicate())
                .state()
                .id;

            // Store the scope id for the next render
            dom.set_mounted_dyn_node(mount, idx, scope_id.0);
        }

        let driver = dom.runtime.get_state(scope_id).render_driver();
        driver.create(dom, scope_id, new, parent, to)
    }
}
