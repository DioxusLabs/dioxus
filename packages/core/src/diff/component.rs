use std::any::{Any, TypeId};

use crate::{
    Element, SuspenseContext,
    any_props::AnyProps,
    diff::{
        anchor::{Anchor, anchor_for_slot, at_anchor},
        context::{DiffContext, DiffFrame, DiffState},
    },
    innerlude::{
        ComponentPropsUpdate, ElementRef, MountId, NoOpMutations, PortalProps, ScopeOrder,
        SuspenseBoundaryProps, SuspenseBoundaryPropsWithOwner, VComponent, WriteMutations,
    },
    nodes::VNode,
    scopes::{LastRenderedNode, ScopeId},
    virtual_dom::VirtualDom,
};

trait ComponentLifecycle {
    fn create<M: WriteMutations>(
        mount: MountId,
        idx: usize,
        component: &VComponent,
        parent: Option<ElementRef>,
        state: &mut DiffState<'_, M>,
    ) -> usize;

    fn diff<M: WriteMutations>(scope_id: ScopeId, state: &mut DiffState<'_, M>);

    fn remove<M: WriteMutations>(
        scope_id: ScopeId,
        state: &mut DiffState<'_, M>,
        destroy_component_state: bool,
    );
}

#[derive(Clone, Copy)]
enum ComponentDriver {
    Normal,
    Portal,
    Suspense,
}

impl ComponentDriver {
    fn from_props(props: &dyn Any) -> Self {
        if props.type_id() == TypeId::of::<SuspenseBoundaryPropsWithOwner>() {
            Self::Suspense
        } else if props.type_id() == TypeId::of::<PortalProps>() {
            Self::Portal
        } else {
            Self::Normal
        }
    }

    fn from_component(component: &VComponent) -> Self {
        Self::from_props(component.props.props())
    }

    fn from_scope(dom: &VirtualDom, scope_id: ScopeId) -> Self {
        Self::from_props(dom.scopes[scope_id.0].props.props())
    }

    fn create<M: WriteMutations>(
        self,
        mount: MountId,
        idx: usize,
        component: &VComponent,
        parent: Option<ElementRef>,
        state: &mut DiffState<'_, M>,
    ) -> usize {
        match self {
            ComponentDriver::Normal => {
                NormalComponentLifecycle::create(mount, idx, component, parent, state)
            }
            ComponentDriver::Portal => {
                PortalLifecycle::create(mount, idx, component, parent, state)
            }
            ComponentDriver::Suspense => {
                SuspenseLifecycle::create(mount, idx, component, parent, state)
            }
        }
    }

    fn diff<M: WriteMutations>(self, scope_id: ScopeId, state: &mut DiffState<'_, M>) {
        match self {
            ComponentDriver::Normal => NormalComponentLifecycle::diff(scope_id, state),
            ComponentDriver::Portal => PortalLifecycle::diff(scope_id, state),
            ComponentDriver::Suspense => SuspenseLifecycle::diff(scope_id, state),
        }
    }

    fn remove<M: WriteMutations>(
        self,
        scope_id: ScopeId,
        state: &mut DiffState<'_, M>,
        destroy_component_state: bool,
    ) {
        match self {
            ComponentDriver::Normal => {
                NormalComponentLifecycle::remove(scope_id, state, destroy_component_state)
            }
            ComponentDriver::Portal => {
                PortalLifecycle::remove(scope_id, state, destroy_component_state)
            }
            ComponentDriver::Suspense => {
                SuspenseLifecycle::remove(scope_id, state, destroy_component_state)
            }
        }
    }
}

struct NormalComponentLifecycle;

impl ComponentLifecycle for NormalComponentLifecycle {
    fn create<M: WriteMutations>(
        mount: MountId,
        idx: usize,
        component: &VComponent,
        parent: Option<ElementRef>,
        state: &mut DiffState<'_, M>,
    ) -> usize {
        let mut scope_id = ScopeId(state.dom.get_mounted_dyn_node(mount, idx));

        // If the scopeid is a placeholder, we need to load up a new scope for this vcomponent. If it's already mounted, then we can just use that
        if scope_id.is_placeholder() {
            scope_id = state
                .dom
                .new_scope(component.props.duplicate(), component.name)
                .state()
                .id;

            // Store the scope id for the next render
            state.dom.set_mounted_dyn_node(mount, idx, scope_id.0);

            // If this is a new scope, we also need to run it once to get the initial state
            let new = state.dom.run_scope(scope_id);

            // Then set the new node as the last rendered node
            state.dom.scopes[scope_id.0].last_rendered_node = Some(LastRenderedNode::new(new));
        }

        let height = state.dom.runtime.get_state(scope_id).height;
        if state
            .dom
            .dirty_fibers
            .remove(&ScopeOrder::new(height, scope_id))
        {
            let mounted = state.dom.scopes[scope_id.0]
                .last_rendered_node
                .as_ref()
                .is_some_and(|node| node.mount.get().mounted());
            if mounted {
                state
                    .dom
                    .run_and_diff_scope(None::<&mut NoOpMutations>, scope_id);
            } else {
                let new = state.dom.run_scope(scope_id);
                state.dom.scopes[scope_id.0].last_rendered_node = Some(LastRenderedNode::new(new));
            }
        }

        let new_node = state.dom.scopes[scope_id.0]
            .last_rendered_node
            .clone()
            .expect("Component to be mounted");

        state
            .dom
            .create_scope(state.to.as_deref_mut(), scope_id, new_node, parent)
    }

    fn diff<M: WriteMutations>(scope_id: ScopeId, state: &mut DiffState<'_, M>) {
        let new_nodes = state.dom.run_scope(scope_id);
        let context = state.context();
        state
            .dom
            .diff_scope(state.to.as_deref_mut(), scope_id, new_nodes, context);
    }

    fn remove<M: WriteMutations>(
        scope_id: ScopeId,
        state: &mut DiffState<'_, M>,
        destroy_component_state: bool,
    ) {
        remove_rendered_scope_node(scope_id, state, destroy_component_state);
    }
}

struct PortalLifecycle;

impl ComponentLifecycle for PortalLifecycle {
    fn create<M: WriteMutations>(
        mount: MountId,
        idx: usize,
        component: &VComponent,
        parent: Option<ElementRef>,
        state: &mut DiffState<'_, M>,
    ) -> usize {
        PortalProps::create(
            mount,
            idx,
            component,
            parent,
            state.dom,
            state.to.as_deref_mut(),
        )
    }

    fn diff<M: WriteMutations>(scope_id: ScopeId, state: &mut DiffState<'_, M>) {
        PortalProps::diff(scope_id, state.dom, state.to.as_deref_mut())
    }

    fn remove<M: WriteMutations>(
        scope_id: ScopeId,
        state: &mut DiffState<'_, M>,
        destroy_component_state: bool,
    ) {
        PortalProps::remove(
            scope_id,
            state.dom,
            state.to.as_deref_mut(),
            destroy_component_state,
        )
    }
}

struct SuspenseLifecycle;

impl ComponentLifecycle for SuspenseLifecycle {
    fn create<M: WriteMutations>(
        mount: MountId,
        idx: usize,
        component: &VComponent,
        parent: Option<ElementRef>,
        state: &mut DiffState<'_, M>,
    ) -> usize {
        SuspenseBoundaryProps::create(
            mount,
            idx,
            component,
            parent,
            state.dom,
            state.to.as_deref_mut(),
        )
    }

    fn diff<M: WriteMutations>(scope_id: ScopeId, state: &mut DiffState<'_, M>) {
        let target_id = state.dom.runtime.get_state(scope_id).target_id();
        let should_write = state.dom.scope_should_write_now(scope_id)
            && state.dom.render_target_should_write(target_id);
        let render_to = if should_write {
            state.to.as_deref_mut()
        } else {
            None
        };
        SuspenseBoundaryProps::diff(scope_id, state.dom, render_to)
    }

    fn remove<M: WriteMutations>(
        scope_id: ScopeId,
        state: &mut DiffState<'_, M>,
        destroy_component_state: bool,
    ) {
        // If this is a suspense boundary, remove the suspended nodes as well.
        //
        // When we are only moving a component out of the real DOM for an
        // ancestor suspense boundary, the nested boundary's suspended nodes are
        // still its background state. Keep them so the nested boundary can
        // resume or continue diffing while hidden.
        if destroy_component_state {
            SuspenseContext::remove_suspended_nodes::<M>(
                state.dom,
                scope_id,
                destroy_component_state,
            );
        }

        remove_rendered_scope_node(scope_id, state, destroy_component_state);
    }
}

fn remove_rendered_scope_node<M: WriteMutations>(
    scope_id: ScopeId,
    state: &mut DiffState<'_, M>,
    destroy_component_state: bool,
) {
    // Remove the component from the dom
    if let Some(node) = state.dom.scopes[scope_id.0].last_rendered_node.clone() {
        node.remove_node_inner(state.dom, state.to.as_deref_mut(), destroy_component_state)
    };

    if destroy_component_state {
        // Now drop all the resources
        state.dom.drop_scope(scope_id);
    }
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
        let driver = ComponentDriver::from_scope(self, scope_id);
        let mut state = DiffState::new_with_context(self, to, parent_context);
        driver.diff(scope_id, &mut state);
    }

    #[tracing::instrument(skip(self, to), level = "trace", name = "VirtualDom::diff_scope")]
    fn diff_scope<M: WriteMutations>(
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

    fn scope_should_write_now(&self, scope: ScopeId) -> bool {
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
        let driver = ComponentDriver::from_scope(self, scope_id);
        let mut state = DiffState::new(self, to);
        driver.remove(scope_id, &mut state, destroy_component_state);
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
        // Replace components that have different render fns
        if old.render_fn != new.render_fn {
            return self.replace_vcomponent(mount, idx, new, parent, state);
        }

        // If the props are static, then we try to memoize by setting the new with the old
        // The target ScopeState still has the reference to the old props, so there's no need to update anything
        // This also implicitly drops the new props since they're not used
        let height = state.dom.runtime.get_state(scope_id).height;

        if let Some(deferred_priority) = state
            .dom
            .render_deferred_priority
            .filter(|priority| *priority > state.dom.render_priority)
        {
            state.dom.queue_component_props_diff(
                deferred_priority,
                vec![ComponentPropsUpdate {
                    scope: scope_id,
                    props: new.props.duplicate(),
                }],
            );
            return;
        }

        if state
            .dom
            .deferred_priority_for_subtree(scope_id, state.dom.render_priority)
            .is_some()
        {
            return;
        }

        // copy out the box for both
        let old_props: &mut dyn AnyProps = &mut *state.dom.scopes[scope_id.0].props;

        if old_props.memoize(new.props.props()) {
            tracing::trace!("Memoized props for component {:#?}", scope_id,);
            return;
        }

        state
            .dom
            .queue_scope(ScopeOrder::with_priority(height, scope_id, state.priority));
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
        ComponentDriver::from_component(component).create(mount, idx, component, parent, state)
    }
}
