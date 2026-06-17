use crate::{
    Element,
    diff::{
        context::{DiffContext, DiffFrame, DiffState},
        placement::{DomAnchor, InsertionSite, at_site, insertion_site_for_slot},
        template::DynamicNodeSlot,
    },
    innerlude::{MountId, MountRef, VComponent, WriteMutations},
    mutations::reborrow_writer,
    nodes::VNode,
    scopes::{LastRenderedNode, MountedOutput, ScopeId},
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
    let result = f(&mut *dom, to);
    result
}

impl VirtualDom {
    /// Run a queued scope diff with an explicit parent diff context.
    ///
    /// Invariant: the scope id is live and its render driver owns the scope's current props and
    /// rendered output.
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
            let old_output = self.scopes[scope.index()]
                .last_rendered_node
                .take()
                .unwrap();
            let old_mount = old_output.root_mount();
            let old = old_output.node();

            // If there are suspended scopes, we need to check if the scope is suspended before we diff it
            // If it is suspended, we need to diff it but write the mutations nothing
            // Note: It is important that we still diff the scope even if it is suspended, because the scope may render other child components which may change between renders
            let mut render_to = to.filter(|_| self.scope_should_write_now(scope));
            let mut state =
                DiffState::new_with_context(self, reborrow_writer(&mut render_to), parent_context);
            let new_mount =
                DiffFrame::new(old_mount, old.as_vnode(), new_real_nodes).diff_into(&mut state);
            self.replace_mounted_component_root_mount(old_mount, new_mount);
            self.runtime
                .get_state(scope)
                .set_root_mount(Some(new_mount));

            self.scopes[scope.index()].last_rendered_node = Some(MountedOutput::new(
                LastRenderedNode::new(new_nodes),
                new_mount,
            ));

            if render_to.is_some() {
                self.runtime.get_state(scope).mount(&self.runtime);
            }
        })
    }

    /// Create or recreate a component scope's rendered output.
    ///
    /// Invariant: the render driver sets `last_rendered_node` and the scope root mount before this
    /// returns. Returns the number of renderer nodes left on the stack.
    #[tracing::instrument(skip(self, to), level = "trace", name = "VirtualDom::create_scope")]
    pub(crate) fn create_scope(
        &mut self,
        to: Option<&mut dyn WriteMutations>,
        scope: ScopeId,
        new_nodes: LastRenderedNode,
        parent: Option<MountRef>,
    ) -> usize {
        self.runtime.clone().with_scope_on_stack(scope, || {
            // If there are suspended scopes, we need to check if the scope is suspended before we diff it
            // If it is suspended, we need to diff it but write the mutations nothing
            // Note: It is important that we still diff the scope even if it is suspended, because the scope may render other child components which may change between renders
            let mut render_to = to.filter(|_| self.scope_should_write_now(scope));

            // Create the node
            let existing_mount = self.scopes[scope.index()]
                .last_rendered_node
                .as_ref()
                .map(MountedOutput::root_mount);
            let created = if let Some(mount) = existing_mount {
                new_nodes.as_vnode().recreate_with_mount(
                    self,
                    mount,
                    parent,
                    parent,
                    reborrow_writer(&mut render_to),
                )
            } else {
                new_nodes.create_with_parents(self, parent, parent, reborrow_writer(&mut render_to))
            };

            // Then set the new node as the last rendered node
            self.scopes[scope.index()].last_rendered_node =
                Some(MountedOutput::new(new_nodes, created.mount));
            self.runtime
                .get_state(scope)
                .set_root_mount(Some(created.mount));

            if render_to.is_some() {
                self.runtime.get_state(scope).mount(&self.runtime);
            }

            created.nodes
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
    /// Diff a dynamic component value in a same-template vnode.
    ///
    /// Invariant: `scope_id` is the component scope mounted in `mount` at `idx`. If the driver
    /// identity changes, replacement owns both new scope creation and old scope removal.
    pub(crate) fn diff_vcomponent(
        &self,
        mount: MountId,
        idx: usize,
        new: &VComponent,
        old: &VComponent,
        scope_id: ScopeId,
        parent: Option<MountRef>,
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
        if scope_driver.memoize(new.driver.as_ref()) {
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
        parent: Option<MountRef>,
        state: &mut DiffState<'_, '_, '_>,
    ) {
        let scope = state
            .dom
            .unchecked_mounted_dynamic_component_scope(mount, idx);

        // Read the old rendered root before freeing the scope slot. If a
        // writer is active, this is the first placement anchor for the new
        // component. Hidden/no-writer diffs do not resolve renderer placement.
        let live_first = state.dom.scopes[scope.index()]
            .last_rendered_node
            .as_ref()
            .and_then(|n| n.mounted_vnode().find_first_element(state.dom));
        let context = state.context();
        let placement_skip = state.placement_skip().to_vec();

        // Free the scope slot so `create_component_node` allocates a new scope.
        state.dom.clear_mounted_dynamic_node_slot(mount, idx);

        if state.to.is_some() {
            let site = live_first
                .map(|id| InsertionSite::AtAnchor(DomAnchor::Before(id)))
                .unwrap_or_else(|| {
                    let anchor = self
                        .template
                        .anchor_for_value(idx)
                        .expect("a dynamic component value always has an owning anchor");
                    let slot = DynamicNodeSlot::new(&self.template, anchor, idx);
                    insertion_site_for_slot(mount, slot, &placement_skip, state.dom, context)
                });
            let runtime = state.dom.runtime.clone();
            let dom = &mut *state.dom;
            let to = reborrow_writer(&mut state.to)
                .expect("writer presence checked before component placement");
            at_site(site, to, runtime, |to| {
                let mut state = DiffState::new_with_context_and_placement_skip(
                    dom,
                    Some(to),
                    context,
                    &placement_skip,
                );
                self.create_component_node(mount, idx, new, parent, &mut state)
            });
        } else {
            self.create_component_node(mount, idx, new, parent, state);
        }
        state
            .dom
            .remove_component_node(reborrow_writer(&mut state.to), true, scope);
    }

    /// Create or reuse the scope for a dynamic component node.
    ///
    /// Invariant: the mounted dynamic slot contains a scope id before driver creation runs, and the
    /// driver writes a rendered root mount into the scope state before returning.
    pub(super) fn create_component_node(
        &self,
        mount: MountId,
        idx: usize,
        component: &VComponent,
        parent: Option<MountRef>,
        state: &mut DiffState<'_, '_, '_>,
    ) -> usize {
        let mut scope_id = state.dom.mounted_dynamic_component_scope(mount, idx);
        if let Some(existing_scope) = scope_id {
            let same_component = {
                let driver = state.dom.runtime.get_state(existing_scope).render_driver();
                driver.same_component(component.driver.as_ref())
            };
            assert!(
                same_component,
                "mounted component scope must match the incoming component driver"
            );
        }

        let new = scope_id.is_none();

        // If the scope id is missing, we need to load up a new scope for this
        // component. Existing scope ids are reusable because the driver
        // identity above proves they belong to this component type.
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
        let root_mount = state
            .dom
            .get_scope(scope_id)
            .expect("component scope must exist after driver creation")
            .last_rendered_node
            .as_ref()
            .expect("component driver creation must set last_rendered_node")
            .root_mount();
        state
            .dom
            .runtime
            .get_state(scope_id)
            .set_root_mount(Some(root_mount));
        nodes
    }
}
