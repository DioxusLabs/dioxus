use std::ops::{Deref, DerefMut};

use crate::{
    any_props::AnyProps,
    innerlude::{ElementRef, FrozenContext, MountId, ScopeOrder, VComponent, WriteMutations},
    nodes::VNode,
    scopes::ScopeId,
    virtual_dom::VirtualDom,
    NoOpMutations,
};

impl VirtualDom {
    pub(crate) fn diff_scope(
        &mut self,
        to: &mut impl WriteMutations,
        scope: ScopeId,
        new_nodes: VNode,
    ) {
        self.runtime.scope_stack.borrow_mut().push(scope);
        let scope_state = &mut self.scopes[scope.0];
        // Load the old and new rendered nodes
        let new = &new_nodes;
        let old = scope_state.last_rendered_node.take().unwrap();

        // If there are suspended scopes, we need to check if the scope is suspended before we diff it
        // If it is suspended, we need to diff it but write the mutations nothing
        // Note: It is important that we still diff the scope even if it is suspended, because the scope may render other child components which may change between renders
        if scope_state
            .state()
            .consume_context::<FrozenContext>()
            .is_some()
        {
            tracing::info!("Rendering suspended scope {scope:?}");
            // old.diff_node(new, self, &mut NoOpMutations);
            todo!()
        } else {
            tracing::info!("Rendering non-suspended scope {scope:?}");
            old.diff_node(new, self, to);
        }

        let scope_state = &mut self.scopes[scope.0];
        scope_state.last_rendered_node = Some(new_nodes);

        self.runtime.scope_stack.borrow_mut().pop();
    }

    /// Create a new template [`VNode`] and write it to the [`WriteMutations`] buffer.
    ///
    /// This method pushes the ScopeID to the internal scopestack and returns the number of nodes created.
    pub(crate) fn create_scope(
        &mut self,
        to: &mut impl WriteMutations,
        scope: ScopeId,
        new_node: VNode,
        parent: Option<ElementRef>,
    ) -> usize {
        self.runtime.scope_stack.borrow_mut().push(scope);

        // Create the node
        let nodes = {
            // If there are suspended scopes, we need to check if the scope is suspended before we diff it
            // If it is suspended, we need to diff it but write the mutations nothing
            // Note: It is important that we still diff the scope even if it is suspended, because the scope may render other child components which may change between renders
            if self.scopes[scope.0]
                .state()
                .consume_context::<FrozenContext>()
                .is_some()
            {
                tracing::info!("Creating suspended scope {scope:?}");
                new_node.create(self, parent);
                // new_node.mount(self, &mut NoOpMutations)
                todo!()
            } else {
                tracing::info!("Creating non-suspended scope {scope:?}");
                new_node.create(self, parent);
                new_node.mount(self, to)
            }
        };

        // Then set the new node as the last rendered node
        self.scopes[scope.0].last_rendered_node = Some(new_node);

        self.runtime.scope_stack.borrow_mut().pop();
        nodes
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
        to: &mut impl WriteMutations,
    ) {
        // Replace components that have different render fns
        if old.render_fn != new.render_fn {
            return self.replace_vcomponent(mount, idx, new, parent, dom, to);
        }

        // copy out the box for both
        let old_scope = &mut dom.scopes[scope_id.0];
        let old_props: &mut dyn AnyProps = old_scope.props.deref_mut();
        let new_props: &dyn AnyProps = new.props.deref();

        // If the props are static, then we try to memoize by setting the new with the old
        // The target ScopeState still has the reference to the old props, so there's no need to update anything
        // This also implicitly drops the new props since they're not used
        if old_props.memoize(new_props.props()) {
            tracing::trace!("Memoized props for component {:#?}", scope_id,);
            return;
        }

        // Now run the component and diff it
        let new = dom.run_scope(scope_id);
        // If the render was successful, diff the new node
        if new.should_render() {
            dom.diff_scope(to, scope_id, new.into());
        }

        let height = dom.runtime.get_state(scope_id).unwrap().height;
        dom.dirty_scopes.remove(&ScopeOrder::new(height, scope_id));
    }

    fn replace_vcomponent(
        &self,
        mount: MountId,
        idx: usize,
        new: &VComponent,
        parent: Option<ElementRef>,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let scope = ScopeId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);

        let m = self.create_component_node(mount, idx, new, parent, dom, to);

        // Instead of *just* removing it, we can use the replace mutation
        dom.remove_component_node(to, scope, Some(m), true);
    }

    pub(super) fn create_component_node(
        &self,
        mount: MountId,
        idx: usize,
        component: &VComponent,
        parent: Option<ElementRef>,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        // Load up a ScopeId for this vcomponent. If it's already mounted, then we can just use that
        let scope = dom
            .new_scope(component.props.duplicate(), component.name)
            .state()
            .id;

        // Store the scope id for the next render
        dom.mounts[mount.0].mounted_dynamic_nodes[idx] = scope.0;

        let new = dom.run_scope(scope);

        dom.create_scope(to, scope, new.into(), parent)
    }
}
