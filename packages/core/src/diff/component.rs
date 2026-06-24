use crate::{
    Element,
    diff::{
        context::{DiffContext, DiffFrame, DiffState},
        placement::{InsertionSite, at_site, insertion_site_for_slot},
        template::DynamicNodeSlot,
    },
    innerlude::{MountId, VComponent, WriteMutations},
    nodes::VNode,
    scopes::{LastRenderedNode, MountedOutput, ScopeId},
    virtual_dom::VirtualDom,
};

impl VirtualDom {
    /// Run a queued scope diff.
    ///
    /// Invariant: the scope id is live and its render driver owns the scope's current props and
    /// rendered output.
    pub(crate) fn run_and_diff_scope(
        &mut self,
        to: Option<&mut (dyn WriteMutations + '_)>,
        scope_id: ScopeId,
    ) {
        let mut state = DiffState::new(self, to);
        let context = state.context();
        let driver = state.dom.runtime.get_state(scope_id).render_driver();
        let to = state.to.as_deref_mut();
        driver.diff(&mut *state.dom, scope_id, context, to)
    }

    #[tracing::instrument(
        skip(self, to, new_nodes, parent_context),
        level = "trace",
        name = "VirtualDom::diff_scope"
    )]
    pub(crate) fn diff_scope(
        &mut self,
        to: Option<&mut (dyn WriteMutations + '_)>,
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
                DiffState::new_with_context(self, render_to.as_deref_mut(), parent_context);
            let new_mount =
                DiffFrame::new(old_mount, old.as_vnode(), new_real_nodes).diff_into(&mut state);
            // `replace_mounted_component_root_mount` keeps this scope's `root_mount`
            // cell in sync, retargeting every cell that holds `old_mount` (this
            // body-driver scope included) to `new_mount`.
            self.replace_mounted_component_root_mount(old_mount, new_mount);

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
        to: Option<&mut (dyn WriteMutations + '_)>,
        scope: ScopeId,
        new_nodes: LastRenderedNode,
        parent: Option<MountId>,
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
                    render_to.as_deref_mut(),
                )
            } else {
                new_nodes.create_with_parents(self, parent, parent, render_to.as_deref_mut())
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
    }

    pub(crate) fn remove_component_node(
        &mut self,
        to: Option<&mut (dyn WriteMutations + '_)>,
        destroy_component_state: bool,
        scope_id: ScopeId,
    ) {
        let driver = self.runtime.get_state(scope_id).render_driver();
        driver.remove(self, scope_id, to, destroy_component_state)
    }
}

impl VNode {
    /// Diff a dynamic component value in a same-template vnode.
    ///
    /// Invariant: `scope_id` is the component scope mounted in `mount` at `idx`. If the driver
    /// identity changes, replacement owns both new scope creation and old scope removal.
    pub(super) fn diff_vcomponent(
        &self,
        mount: MountId,
        slot: DynamicNodeSlot<'_>,
        new: &VComponent,
        old: &VComponent,
        scope_id: ScopeId,
        state: &mut DiffState<'_, '_, '_, '_>,
    ) {
        // Replace components whose render function or specialized lifecycle driver changed.
        if old.render_fn != new.render_fn || !old.driver.same_component(&*new.driver) {
            return self.replace_vcomponent(mount, slot, new, state);
        }

        // If the props are static, then we try to memoize by setting the new with the old. The
        // target ScopeState still has the old props, so a true return means there is no need to
        // update anything. This also implicitly drops the new props since they are not used.
        let old_scope = &mut state.dom.scopes[scope_id.index()];
        if old_scope.props.memoize(new.props.props()) {
            return;
        }

        let context = state.context();
        let driver = state.dom.runtime.get_state(scope_id).render_driver();
        let to = state.to.as_deref_mut();
        driver.diff(&mut *state.dom, scope_id, context, to);
        state.dom.mark_clean(scope_id);
    }

    fn replace_vcomponent(
        &self,
        mount: MountId,
        slot: DynamicNodeSlot<'_>,
        new: &VComponent,
        state: &mut DiffState<'_, '_, '_, '_>,
    ) {
        let idx = slot.index();
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
        let site = state.has_writer().then(|| {
            live_first
                .map(InsertionSite::before)
                .unwrap_or_else(|| insertion_site_for_slot(mount, slot, state.dom, context))
        });

        // Free the scope slot so `create_component_node` allocates a new scope.
        state.dom.clear_mounted_dynamic_node_slot(mount, idx);

        if let Some(site) = site {
            let runtime = state.dom.runtime.clone();
            let dom = &mut *state.dom;
            let to = state.to.as_deref_mut().expect("writer checked");
            at_site(site, to, runtime, |to| {
                let mut state = DiffState::new_with_context(dom, Some(to), context);
                self.create_component_node(mount, idx, new, &mut state)
            });
        } else {
            self.create_component_node(mount, idx, new, state);
        }
        state
            .dom
            .remove_component_node(state.to.as_deref_mut(), true, scope);
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
        state: &mut DiffState<'_, '_, '_, '_>,
    ) -> usize {
        let mut scope_id = state.dom.mounted_dynamic_component_scope(mount, idx);

        let new = scope_id.is_none();

        // If the scope id is missing, we need to load up a new scope for this
        // component. Existing scope ids are reusable because the mounted
        // dynamic slot invariant guarantees they belong to this component type.
        if new {
            // The scope adopts a duplicate of the vnode's driver so the live
            // scope never aliases props with a vnode (a cached rsx element
            // hands out the same driver instance every render).
            let new_scope_id = state
                .dom
                .new_scope(
                    component.name,
                    component.driver.clone(),
                    component.props.duplicate(),
                )
                .state()
                .id;
            scope_id = Some(new_scope_id);

            // Store the scope id for the next render
            state
                .dom
                .set_mounted_dynamic_component_scope(mount, idx, new_scope_id);
        }

        let scope_id = scope_id.expect("component mounted");
        let driver = state.dom.runtime.get_state(scope_id).render_driver();
        let to = state.to.as_deref_mut();
        let parent = Some(mount);
        let nodes = driver.create(&mut *state.dom, scope_id, new, parent, to);
        let root_mount = state
            .dom
            .get_scope(scope_id)
            .expect("component scope")
            .last_rendered_node
            .as_ref()
            .expect("component root")
            .root_mount();
        state
            .dom
            .runtime
            .get_state(scope_id)
            .set_root_mount(Some(root_mount));
        nodes
    }
}
