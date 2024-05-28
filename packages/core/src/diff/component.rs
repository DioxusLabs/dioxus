use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

use crate::{
    any_props::AnyProps,
    innerlude::{
        ElementRef, FrozenContext, MountId, ScopeOrder, SuspenseBoundaryProps,
        SuspenseBoundaryPropsWithOwner, VComponent, WriteMutations,
    },
    nodes::VNode,
    scopes::ScopeId,
    virtual_dom::VirtualDom,
    RenderReturn,
};

impl VirtualDom {
    pub(crate) fn run_and_diff_scope<M: WriteMutations>(
        &mut self,
        to: Option<&mut M>,
        scope_id: ScopeId,
    ) {
        let scope = &mut self.scopes[scope_id.0];
        println!(
            "running {:?} vs {:?}",
            scope.props.props().type_id(),
            TypeId::of::<SuspenseBoundaryPropsWithOwner>()
        );
        if SuspenseBoundaryProps::downcast_from_props(&mut *scope.props).is_some() {
            SuspenseBoundaryProps::diff(scope_id, self, to)
        } else {
            let new_nodes = self.run_scope(scope_id);
            self.diff_scope(to, scope_id, new_nodes);
        }
    }

    fn diff_scope<M: WriteMutations>(
        &mut self,
        to: Option<&mut M>,
        scope: ScopeId,
        new_nodes: RenderReturn,
    ) {
        // We don't diff the nodes if the scope is suspended or has an error
        let Ok(new_real_nodes) = &new_nodes.node else {
            return;
        };

        self.runtime.scope_stack.borrow_mut().push(scope);
        let scope_state = &mut self.scopes[scope.0];
        // Load the old and new rendered nodes
        let old = scope_state.last_rendered_node.take().unwrap();

        // If there are suspended scopes, we need to check if the scope is suspended before we diff it
        // If it is suspended, we need to diff it but write the mutations nothing
        // Note: It is important that we still diff the scope even if it is suspended, because the scope may render other child components which may change between renders
        let render_to = to.filter(|_| !self.scope_frozen(scope, old.suspended()));
        old.diff_node(new_real_nodes, self, render_to);

        self.scopes[scope.0].last_rendered_node = Some(new_nodes);

        self.runtime.scope_stack.borrow_mut().pop();
    }

    fn scope_frozen(&self, scope: ScopeId, return_suspended: bool) -> bool {
        // We only freeze rendering on the scope if:
        // - There are suspended scopes (this is very cheap to check) and more than just this scope is suspended
        // - This scope is under a suspended scope with tasks spawned outside of this scope
        self.runtime.suspended_tasks.get() > return_suspended as usize
            && self.scopes[scope.0]
                .state()
                .consume_context::<FrozenContext>()
                .filter(|suspense| suspense.suspended_tasks_outside_of_scope(scope))
                .is_some()
    }

    /// Create a new [`ScopeState`] for a component that has been created with [`VirtualDom::create_scope`]
    ///
    /// Returns the number of nodes created on the stack
    pub(crate) fn create_scope<M: WriteMutations>(
        &mut self,
        to: Option<&mut M>,
        scope: ScopeId,
        new_nodes: RenderReturn,
        parent: Option<ElementRef>,
    ) -> usize {
        self.runtime.scope_stack.borrow_mut().push(scope);

        // If there are suspended scopes, we need to check if the scope is suspended before we diff it
        // If it is suspended, we need to diff it but write the mutations nothing
        // Note: It is important that we still diff the scope even if it is suspended, because the scope may render other child components which may change between renders
        let render_to = to.filter(|_| !self.scope_frozen(scope, new_nodes.suspended()));

        // Create the node
        let nodes = new_nodes.create(self, parent, render_to);

        // Then set the new node as the last rendered node
        self.scopes[scope.0].last_rendered_node = Some(new_nodes);

        self.runtime.scope_stack.borrow_mut().pop();
        nodes
    }

    pub(crate) fn remove_component_node<M: WriteMutations>(
        &mut self,
        to: Option<&mut M>,
        destroy_component_state: bool,
        scope_id: ScopeId,
        replace_with: Option<usize>,
    ) {
        // If this is a suspense boundary, remove the suspended nodes as well
        if let Some(suspense) = self.scopes[scope_id.0]
            .props
            .props_mut()
            .downcast_mut::<SuspenseBoundaryProps>()
        {
            // Remove the suspended nodes
            if let Some(node) = suspense.suspended_nodes.take() {
                node.remove_node_inner(self, None::<&mut M>, destroy_component_state, None)
            }
        }

        // Remove the component from the dom
        if let Some(node) = self.scopes[scope_id.0].last_rendered_node.as_ref() {
            node.clone_mounted()
                .remove_node_inner(self, to, destroy_component_state, replace_with)
        };

        if destroy_component_state {
            // Now drop all the resources
            self.drop_scope(scope_id);
        }
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

        // Now diff the scope
        dom.run_and_diff_scope(to, scope_id);

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
        dom.remove_component_node(to, true, scope, Some(m));
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
        // If this is a suspense boundary, run our suspense creation logic instead of running the component
        if component.props.props().type_id() == TypeId::of::<SuspenseBoundaryPropsWithOwner>() {
            return SuspenseBoundaryProps::create(mount, idx, component, parent, dom, to);
        }

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
            dom.scopes[scope_id.0].last_rendered_node = Some(new);
        }

        let scope = ScopeId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);

        let new_node = dom.scopes[scope.0]
            .last_rendered_node
            .as_ref()
            .expect("Component to be mounted")
            .clone();

        dom.create_scope(to, scope, new_node, parent)
    }
}
