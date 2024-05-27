use std::ops::{Deref, DerefMut};

use crate::{
    any_props::AnyProps,
    innerlude::{ElementRef, FrozenContext, MountId, ScopeOrder, VComponent, WriteMutations},
    nodes::VNode,
    scopes::ScopeId,
    virtual_dom::VirtualDom,
};

impl VirtualDom {
    pub(crate) fn diff_scope<M: WriteMutations>(
        &mut self,
        mut to: Option<&mut M>,
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
        let frozen = scope_state
            .state()
            .consume_context::<FrozenContext>()
            .is_some();
        if frozen {
            tracing::info!("Rendering suspended scope {scope:?}");
            old.diff_node(new, self, None::<&mut M>);
        } else {
            tracing::info!("Rendering non-suspended scope {scope:?}");
            old.diff_node(new, self, to.as_deref_mut());

            if to.is_some() {
                self.scopes[scope.0].last_mounted_node = Some(new_nodes.clone_mounted());
            }
        }

        self.scopes[scope.0].last_rendered_node = Some(new_nodes.clone_mounted());

        self.runtime.scope_stack.borrow_mut().pop();
    }

    /// Create a new [`ScopeState`] for a component that has been created with [`VirtualDom::create_scope`]
    ///
    /// Returns the number of nodes created on the stack
    pub(crate) fn create_scope<M: WriteMutations>(
        &mut self,
        mut to: Option<&mut M>,
        scope: ScopeId,
        new_nodes: VNode,
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
                // Suspended nodes don't get mounted, so we don't pass down the mutations
                new_nodes.create(self, parent, Option::<&mut M>::None)
            } else {
                tracing::info!("Creating non-suspended scope {scope:?}");
                let nodes_created = new_nodes.create(self, parent, to.as_deref_mut());

                if to.is_some() {
                    self.scopes[scope.0].last_mounted_node = Some(new_nodes.clone_mounted());
                }

                nodes_created
            }
        };

        // Then set the new node as the last rendered node
        self.scopes[scope.0].last_rendered_node = Some(new_nodes);

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
        to: Option<&mut impl WriteMutations>,
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
        if new.should_diff() {
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
        mut to: Option<&mut impl WriteMutations>,
    ) {
        let scope = ScopeId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);

        // Remove the scope id from the mount
        dom.mounts[mount.0].mounted_dynamic_nodes[idx] = ScopeId::PLACEHOLDER.0;
        let m = self.create_component_node(mount, idx, new, parent, dom, to.as_deref_mut());

        // Instead of *just* removing it, we can use the replace mutation
        dom.remove_component_node(to, scope, Some(m));
    }

    /// Create a new component (if it doesn't already exist) node and then mount the [`ScopeState`] for a component
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
        let mut scope_id = ScopeId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);

        // If the scopeid is a placeholder, we need to load up a new scope for this vcomponent. If it's already mounted, then we can just use that
        if scope_id.is_placeholder() {
            scope_id = dom
                .new_scope(component.props.duplicate(), component.name)
                .state()
                .id;

            // Store the scope id for the next render
            dom.mounts[mount.0].mounted_dynamic_nodes[idx] = scope_id.0;

            // If this is a new scope, we also need to run it once to get the initial state
            let new = dom.run_scope(scope_id);

            // Then set the new node as the last rendered node
            dom.scopes[scope_id.0].last_rendered_node = Some(new.clone_mounted());
        }

        let scope = ScopeId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);

        let new_node = dom.scopes[scope.0]
            .last_rendered_node
            .as_ref()
            .expect("Component to be mounted")
            .clone_mounted();

        dom.create_scope(to, scope, new_node, parent)
    }
}
