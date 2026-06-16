use crate::{
    DynamicNode::*,
    Template, TemplatePath, VNode, VirtualDom, WriteMutations,
    arena::{ElementId, MountedElementId},
    diff::{
        context::{DiffFrame, DiffState},
        placement::{
            DomAnchor, ElementEdge, InsertionSite, at_site, create_at_site, insertion_site_at,
            insertion_site_for_slot,
        },
        template::{
            DynamicAttrGroup, DynamicNodeSlot, TemplateRoot, dynamic_node_slots,
            for_each_dynamic_attr_group, template_roots,
        },
    },
    innerlude::{MountId, MountRef},
    mutations::{reborrow_writer, remove_id, with_consumed_id, with_id},
    nodes::DynamicNode,
    scopes::ScopeId,
};

impl VNode {
    pub(crate) fn diff_node(
        &self,
        new: &VNode,
        dom: &mut VirtualDom,
        to: Option<&mut dyn WriteMutations>,
    ) {
        let mut state = DiffState::new(dom, to);
        DiffFrame::new(self.unchecked_mounted_id(), self, new).diff_into(&mut state);
    }
}

impl<'a> DiffFrame<'a> {
    pub(crate) fn diff_into(self, state: &mut DiffState<'_, '_, '_>) {
        let old = self.old;
        let new = self.new;

        let current_mount = self.mount;
        let writes_enabled = state.dom.mount_should_render(current_mount) && state.to.is_some();
        let mut state = state.reborrow_with_writes(writes_enabled);

        // If the templates are different, we need to replace the entire template
        if old.template != new.template {
            let parent = state.dom.get_mounted_parent(current_mount);
            return old.replace_inner(std::slice::from_ref(new), parent, &mut state, true);
        }

        let prev_mount = state.dom.claim_mount(old, new);
        state.enter_context(prev_mount, old, new);

        // If the templates are the same, we don't need to do anything, except copy over the mount information
        if old == new {
            state.dom.commit_mount(prev_mount, new);
            return;
        }

        // If the templates are the same, we can diff the attributes and children
        // Start with the attributes
        // Since the attributes are only side effects, we can skip diffing them entirely if the node is suspended and we aren't outputting mutations
        if let Some(to) = reborrow_writer(&mut state.to) {
            old.diff_attributes(new, state.dom, to);
        }

        let mount_id = new.unchecked_mounted_id();
        for slot in dynamic_node_slots(old) {
            let dyn_node_idx = slot.index();
            old.diff_dynamic_node(
                mount_id,
                slot,
                old.dynamic_values[dyn_node_idx].node(),
                new.dynamic_values[dyn_node_idx].node(),
                &mut state,
            );
        }
        state.dom.commit_mount(mount_id, new);
    }
}

impl VNode {
    fn diff_dynamic_node(
        &self,
        mount: MountId,
        slot: DynamicNodeSlot<'_>,
        old_node: &DynamicNode,
        new_node: &DynamicNode,
        state: &mut DiffState<'_, '_, '_>,
    ) {
        let idx = slot.index();
        match (old_node, new_node) {
            (Text(old), Text(new)) => {
                // Diffing text is just a side effect, if we are diffing suspended nodes and are not outputting mutations, we can skip it
                if let Some(to) = reborrow_writer(&mut state.to)
                    && old.value != new.value
                {
                    let id = state
                        .dom
                        .unchecked_mounted_dynamic_text_node(mount, idx)
                        .element_id();
                    with_id(to, id, |to| to.set_text(&new.value));
                }
            }
            (Fragment(old), Fragment(new)) => self.diff_fragment(mount, slot, old, new, state),
            (Component(old), Component(new)) => {
                let scope_id = state
                    .dom
                    .unchecked_mounted_dynamic_component_scope(mount, idx);
                self.diff_vcomponent(
                    mount,
                    idx,
                    new,
                    old,
                    scope_id,
                    Some(MountRef { mount }),
                    state,
                )
            }
            (old, new) => self.replace_dynamic_node_at_slot(mount, slot, old, new, state),
        };
    }

    fn replace_dynamic_node_at_slot(
        &self,
        mount: MountId,
        slot: DynamicNodeSlot<'_>,
        old: &DynamicNode,
        new: &DynamicNode,
        state: &mut DiffState<'_, '_, '_>,
    ) {
        let idx = slot.index();
        let old_has_live_dom = self.dynamic_node_has_live_dom(mount, idx, old, state.dom);
        if !old_has_live_dom {
            // Pass `None::<&mut M>` (the caller's writer type) instead of
            // `NoOpMutations` so this call reuses the caller's monomorphization.
            // A `NoOpMutations` mono here would carry copies of every
            // generic-driven function it transitively calls — `reclaim_roots`,
            // `remove_node_inner`, etc. — whose "writes enabled" arms are
            // unreachable in the NoOp mono, and that tanks per-monomorphization
            // region coverage.
            self.remove_dynamic_node(mount, state.dom, None, true, idx, old);
        }

        let live_first = if old_has_live_dom {
            self.dynamic_node_first_element(mount, idx, old, state.dom)
        } else {
            None
        };
        let site = match live_first {
            Some(first) => InsertionSite::AtAnchor(DomAnchor::Before(first)),
            None => insertion_site_for_slot(mount, slot, &[], state.dom, state.context()),
        };

        state.with_mounted_dynamic_node_slot_replaced(
            mount,
            idx,
            old_has_live_dom,
            |state| {
                let runtime = state.dom.runtime.clone();
                let dom = &mut *state.dom;
                let to = reborrow_writer(&mut state.to);
                at_site(site, to, runtime, |to| {
                    let mut state = DiffState::new(dom, to);
                    self.create_dynamic_node(new, mount, idx, &mut state)
                });
            },
            |state| {
                self.remove_dynamic_node(
                    mount,
                    state.dom,
                    reborrow_writer(&mut state.to),
                    true,
                    idx,
                    old,
                );
            },
        );
    }

    /// Diff two fragments at a dynamic slot. Handles empty <-> non-empty transitions
    /// without using placeholders to preserve the slot position.
    fn diff_fragment(
        &self,
        mount: MountId,
        slot: DynamicNodeSlot<'_>,
        old: &[VNode],
        new: &[VNode],
        state: &mut DiffState<'_, '_, '_>,
    ) {
        let parent = Some(MountRef { mount });
        match (old.is_empty(), new.is_empty()) {
            (true, true) => {}
            (true, false) => {
                // Empty → non-empty: stage new content at the slot insertion site.
                let own_mounts: Vec<MountId> = new.iter().filter_map(VNode::mounted_id).collect();
                let site =
                    insertion_site_for_slot(mount, slot, &own_mounts, state.dom, state.context());
                create_at_site(new, parent, site, state.dom, reborrow_writer(&mut state.to));
            }
            (false, true) => {
                state.dom.remove_nodes(reborrow_writer(&mut state.to), old);
            }
            (false, false) => {
                state.diff_non_empty_fragment(old, new, parent);
            }
        }
    }

    pub(crate) fn find_first_element(&self, dom: &VirtualDom) -> Option<ElementId> {
        self.find_element_in_roots(dom, dom.current_render_target_id(), ElementEdge::First)
    }

    fn find_element_at_root_in_target(
        &self,
        root_idx: usize,
        mount: MountId,
        target_id: crate::RenderTargetId,
        dom: &VirtualDom,
    ) -> Option<ElementId> {
        if dom.mount_target_id(mount) != target_id {
            return None;
        }
        if root_idx >= dom.mounted_root_count(mount) {
            return None;
        }
        dom.mounted_root_node(mount, root_idx)
            .filter(|id| dom.element_exists_in_target(target_id, *id))
            .map(MountedElementId::element_id)
    }

    pub(crate) fn find_last_element(&self, dom: &VirtualDom) -> Option<ElementId> {
        self.find_element_in_roots(dom, dom.current_render_target_id(), ElementEdge::Last)
    }

    fn has_live_dom(&self, dom: &VirtualDom) -> bool {
        let Some(mount) = self.mounted_id() else {
            return false;
        };
        if (0..self.template.root_count()).any(|root_idx| {
            root_idx < dom.mounted_root_count(mount) && self.root_has_live_dom(root_idx, mount, dom)
        }) {
            return true;
        }

        dynamic_node_slots(self).any(|slot| {
            if !slot.is_root_level() {
                return false;
            }

            let idx = slot.index();
            idx < dom.mounted_dyn_node_count(mount)
                && self.dynamic_node_has_live_dom(mount, idx, self.dynamic_values[idx].node(), dom)
        })
    }

    fn root_has_live_dom(&self, root_idx: usize, mount: MountId, dom: &VirtualDom) -> bool {
        // Count checks keep stale vnode templates from indexing past the
        // slots that were allocated for this live mount.
        dom.mounted_root_node(mount, root_idx)
            .is_some_and(|id| dom.element_exists_for_mount(mount, id))
    }

    fn find_element_in_roots(
        &self,
        dom: &VirtualDom,
        target_id: crate::RenderTargetId,
        edge: ElementEdge,
    ) -> Option<ElementId> {
        let mount = self.mounted_id()?;
        match edge {
            ElementEdge::First => (0..self.template.root_count()).find_map(|cursor_idx| {
                self.find_root_dynamic_at_cursor(cursor_idx, mount, target_id, dom, edge)
                    .or_else(|| {
                        self.find_element_at_root_in_target(cursor_idx, mount, target_id, dom)
                    })
            }),
            ElementEdge::Last => (0..self.template.root_count()).rev().find_map(|root_idx| {
                self.find_element_at_root_in_target(root_idx, mount, target_id, dom)
                    .or_else(|| {
                        self.find_root_dynamic_at_cursor(root_idx, mount, target_id, dom, edge)
                    })
            }),
        }
    }

    fn find_root_dynamic_at_cursor(
        &self,
        cursor_idx: usize,
        mount: MountId,
        target_id: crate::RenderTargetId,
        dom: &VirtualDom,
        edge: ElementEdge,
    ) -> Option<ElementId> {
        let mut found = None;
        match edge {
            ElementEdge::First => {
                for slot in dynamic_node_slots(self) {
                    if !slot.is_root_level() || slot.root_index() != cursor_idx {
                        continue;
                    }

                    let idx = slot.index();
                    found = self.dynamic_node_edge_element(
                        mount,
                        idx,
                        self.dynamic_values[idx].node(),
                        dom,
                        target_id,
                        ElementEdge::First,
                    );
                    if found.is_some() {
                        break;
                    }
                }
                found
            }
            ElementEdge::Last => {
                for slot in dynamic_node_slots(self) {
                    if slot.is_root_level() && slot.root_index() == cursor_idx {
                        let idx = slot.index();
                        found = self.dynamic_node_edge_element(
                            mount,
                            idx,
                            self.dynamic_values[idx].node(),
                            dom,
                            target_id,
                            ElementEdge::Last,
                        );
                    }
                }
                found
            }
        }
    }

    pub(crate) fn replace(
        &self,
        right: &[VNode],
        parent: Option<MountRef>,
        dom: &mut VirtualDom,
        to: Option<&mut dyn WriteMutations>,
    ) {
        let mut state = DiffState::new(dom, to);
        self.replace_inner(right, parent, &mut state, true)
    }

    /// Replace this node with new children, but *don't destroy* the old node's component state
    ///
    /// This is useful for moving a node from the rendered nodes into a suspended node
    pub(crate) fn move_node_to_background(
        &self,
        right: &[VNode],
        parent: Option<MountRef>,
        dom: &mut VirtualDom,
        to: Option<&mut dyn WriteMutations>,
    ) {
        let mut state = DiffState::new(dom, to);
        self.replace_inner(right, parent, &mut state, false)
    }

    pub(crate) fn replace_inner(
        &self,
        right: &[VNode],
        parent: Option<MountRef>,
        state: &mut DiffState<'_, '_, '_>,
        destroy_component_state: bool,
    ) {
        let own_mounts: Vec<MountId> = right.iter().filter_map(VNode::mounted_id).collect();
        // When the old subtree has no live DOM and the boundary is hidden, we
        // skip emitting renderer mutations for both the create and remove
        // sides. We still call `create_at_site` so the new subtree gets its
        // mount slots populated — otherwise the caller (e.g. suspense's
        // background diff) may later read a mount that was never set.
        let suppress_mutations = self.should_suppress_mutations(state.dom, destroy_component_state);
        let site = insertion_site_at(
            ElementEdge::First,
            self,
            &own_mounts,
            state.dom,
            state.context(),
        );
        let mut to_for_create = reborrow_writer(&mut state.to);
        if suppress_mutations {
            to_for_create = None;
        }
        create_at_site(right, parent, site, state.dom, to_for_create);
        let to_for_remove = if suppress_mutations {
            None
        } else {
            reborrow_writer(&mut state.to)
        };
        self.remove_node_inner(state.dom, to_for_remove, destroy_component_state);
    }

    /// True when we may skip emitting renderer mutations for a replace because
    /// the old subtree has no live DOM and we're operating inside a suspended
    /// boundary (or have no `WriteMutations` sink at all).
    fn should_suppress_mutations(&self, dom: &VirtualDom, destroy_component_state: bool) -> bool {
        if !destroy_component_state {
            return false;
        }
        if self.has_live_dom(dom) {
            return false;
        }
        current_scope_hidden_by_suspense(dom) && self.has_reclaimable_root()
    }

    fn has_reclaimable_root(&self) -> bool {
        dynamic_node_slots(self).any(|slot| {
            let id = slot.index();
            slot.is_root_level()
                && matches!(self.dynamic_values[id].node(), Text(text) if text.value.is_empty())
        })
    }

    /// Remove a node from the dom.
    pub(crate) fn remove_node(&self, dom: &mut VirtualDom, to: Option<&mut dyn WriteMutations>) {
        self.remove_node_inner(dom, to, true)
    }

    /// Remove a node, but only maybe destroy the component state of that node. During suspense, we need to remove a node from the real dom without wiping the component state
    pub(crate) fn remove_node_inner(
        &self,
        dom: &mut VirtualDom,
        to: Option<&mut dyn WriteMutations>,
        destroy_component_state: bool,
    ) {
        // Every caller (replace_inner, remove_nodes, Fragment removal,
        // scope cleanup) only reaches here with vnodes that went through
        // `create_with_parents` and have live mount slots in the mount
        // registry. `build_vnode` / `claim_mount` always assign a live
        // MountId before anything tries to remove it.
        let mount = self.unchecked_mounted_id();

        // Clean up any attributes that have claimed a static node as dynamic for mount/unmounts
        // Will not generate mutations!
        self.reclaim_attributes(mount, dom);

        // Remove the nested dynamic nodes
        // We don't generate mutations for these, as they will be removed by the parent (in the next line)
        // But we still need to make sure to reclaim them from the arena and drop their hooks, etc
        self.remove_nested_dyn_nodes(mount, dom, destroy_component_state);

        // Clean up the roots, assuming we need to generate mutations for these
        // This is done last in order to preserve Node ID reclaim order (reclaim in reverse order of claim)
        self.reclaim_roots(mount, dom, to, destroy_component_state);

        if destroy_component_state {
            let mount = self.take_mounted_id();
            dom.remove_mount(mount);
        }
    }

    fn reclaim_roots(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        mut to: Option<&mut dyn WriteMutations>,
        destroy_component_state: bool,
    ) {
        for slot in dynamic_node_slots(self) {
            let id = slot.index();
            if slot.is_root_level() {
                let dynamic_node = self.dynamic_values[id].node();
                // Empty Fragments contribute no DOM and have nothing to reclaim
                // via the renderer — skip them entirely.
                if matches!(dynamic_node, DynamicNode::Fragment(nodes) if nodes.is_empty()) {
                    continue;
                }
                self.remove_dynamic_node(
                    mount,
                    dom,
                    reborrow_writer(&mut to),
                    destroy_component_state,
                    id,
                    dynamic_node,
                );
            }
        }

        for idx in 0..self.template.root_count() {
            let Some(id) = dom.mounted_root_node(mount, idx) else {
                // Already reclaimed during a previous `move_node_to_background`.
                continue;
            };
            if let Some(to) = reborrow_writer(&mut to) {
                remove_id(to, id.element_id());
            }
            dom.reclaim_for_mount(mount, id);
            dom.clear_mounted_root_node(mount, idx);
        }
    }

    fn remove_nested_dyn_nodes(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        destroy_component_state: bool,
    ) {
        for slot in dynamic_node_slots(self) {
            let idx = slot.index();
            let dyn_node = self.dynamic_values[idx].node();
            // Roots are cleaned up automatically above; non-root nested dynamic nodes get cleaned here.
            if !slot.is_root_level() {
                self.remove_dynamic_node(mount, dom, None, destroy_component_state, idx, dyn_node)
            }
        }
    }

    fn remove_dynamic_node(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        mut to: Option<&mut dyn WriteMutations>,
        destroy_component_state: bool,
        idx: usize,
        node: &DynamicNode,
    ) {
        match node {
            Component(_comp) => {
                let scope_id = dom.unchecked_mounted_dynamic_component_scope(mount, idx);
                dom.remove_component_node(to, destroy_component_state, scope_id);
            }
            Text(_) => {
                let Some(id) = dom.mounted_dynamic_text_node(mount, idx) else {
                    // No DOM was ever materialized for this text (e.g. it was rendered
                    // into a background-suspended subtree) or it was already reclaimed
                    // via a prior `move_node_to_background`. Skip emission/reclaim.
                    return;
                };
                if let Some(to) = to {
                    remove_id(to, id.element_id());
                }
                dom.reclaim_for_mount(mount, id);
                dom.clear_mounted_dynamic_text_node(mount, idx);
            }
            Fragment(nodes) => {
                for node in nodes.iter() {
                    node.remove_node_inner(dom, reborrow_writer(&mut to), destroy_component_state);
                }
            }
        };
    }

    fn dynamic_node_has_live_dom(
        &self,
        mount: MountId,
        idx: usize,
        node: &DynamicNode,
        dom: &VirtualDom,
    ) -> bool {
        match node {
            Component(_) => {
                let scope_id = dom.unchecked_mounted_dynamic_component_scope(mount, idx);
                dom.get_scope(scope_id)
                    .and_then(|scope| scope.try_root_node())
                    .is_some_and(|node| node.has_live_dom(dom))
            }
            Text(_) => dom
                .mounted_dynamic_text_node(mount, idx)
                .is_some_and(|id| dom.element_exists_for_mount(mount, id)),
            Fragment(nodes) => nodes.iter().any(|node| node.has_live_dom(dom)),
        }
    }

    fn dynamic_node_first_element(
        &self,
        mount: MountId,
        idx: usize,
        node: &DynamicNode,
        dom: &VirtualDom,
    ) -> Option<ElementId> {
        let target_id = dom.current_render_target_id();
        match node {
            Component(_) => {
                // The only caller (`replace_dynamic_node_at_slot`) gates this
                // entire call on `old_has_live_dom` returning true, and
                // `dynamic_node_has_live_dom` for `Component` is true only
                // after `get_scope(_).and_then(try_root_node).is_some_and(...)`
                // already returned true. So the scope is live and rendered
                // by the time we get here.
                let scope_id = dom.unchecked_mounted_dynamic_component_scope(mount, idx);
                let root = live_component_root(dom, scope_id);
                root.find_element_in_roots(dom, target_id, ElementEdge::First)
            }
            Text(_) => dom
                .mounted_dynamic_text_node(mount, idx)
                .filter(|id| dom.element_exists_in_target(target_id, *id))
                .map(MountedElementId::element_id),
            Fragment(nodes) => find_fragment_edge(nodes, dom, target_id, ElementEdge::First),
        }
    }

    fn dynamic_node_edge_element(
        &self,
        mount: MountId,
        idx: usize,
        node: &DynamicNode,
        dom: &VirtualDom,
        target_id: crate::RenderTargetId,
        edge: ElementEdge,
    ) -> Option<ElementId> {
        match node {
            Component(_) => {
                let scope_id = dom.unchecked_mounted_dynamic_component_scope(mount, idx);
                live_component_root(dom, scope_id).find_element_in_roots(dom, target_id, edge)
            }
            Text(_) if dom.mount_target_id(mount) == target_id => dom
                .mounted_dynamic_text_node(mount, idx)
                .filter(|id| dom.element_exists_in_target(target_id, *id))
                .map(MountedElementId::element_id),
            Text(_) => None,
            Fragment(nodes) => find_fragment_edge(nodes, dom, target_id, edge),
        }
    }

    pub(super) fn reclaim_attributes(&self, mount: MountId, dom: &mut VirtualDom) {
        let mut next_id = None;
        for_each_dynamic_attr_group(self, |group| {
            // We clean up the roots in the next step, so don't worry about them here
            if group.path().is_root_level_slot() {
                return;
            }

            // only reclaim the new element if it's different from the previous one
            for idx in group.ids() {
                let new_id = dom.mounted_dyn_attr(mount, idx);
                if let Some(new_id) = new_id
                    && Some(new_id) != next_id
                {
                    dom.reclaim_for_mount(mount, new_id);
                    next_id = Some(new_id);
                }
                dom.clear_mounted_dyn_attr(mount, idx);
            }
        });
    }

    /// Create this rsx block. This will create scopes from components that this rsx block contains, but it will not write anything to the DOM.
    pub(crate) fn create(
        &self,
        dom: &mut VirtualDom,
        parent: Option<MountRef>,
        to: Option<&mut dyn WriteMutations>,
    ) -> usize {
        self.create_with_parents(dom, parent, parent, to)
    }

    /// Create this rsx block with separate renderer and logical parents.
    pub(crate) fn create_with_parents(
        &self,
        dom: &mut VirtualDom,
        render_parent: Option<MountRef>,
        logical_parent: Option<MountRef>,
        to: Option<&mut dyn WriteMutations>,
    ) -> usize {
        let mut state = DiffState::new(dom, to);
        // Get the most up to date template
        let template = self.template;

        // Initialize the mount information for this vnode if it isn't already mounted
        if self.mounted_id().is_none() {
            state.dom.create_mount(
                self,
                render_parent,
                logical_parent,
                template.root_count(),
                template.dynamics().len(),
                template.dynamics().len(),
            );
        }

        // Get the mounted id of this block
        // At this point, we should have already mounted the block
        let mount = self.unchecked_mounted_id();
        if !state.dom.mount_should_render(mount) {
            state.to = None;
        }

        let mut nodes_created = 0;
        for root in template_roots(self) {
            if let TemplateRoot::Static {
                root_idx,
                op: root_op,
            } = root
            {
                let writes_enabled = state.to.is_some();
                if let Some(to) = reborrow_writer(&mut state.to) {
                    self.load_template_root(mount, root_idx, root_op, state.dom, to);
                }

                if template.enter_meta(root_op).is_some() {
                    let root_path = TemplatePath::root(root_idx);
                    if let Some(to) = reborrow_writer(&mut state.to) {
                        self.write_attrs_for_static_node(mount, root_path, state.dom, to);
                    }
                    self.load_dynamic_slots_for_static_node(mount, root_path, &mut state);
                }

                nodes_created += usize::from(writes_enabled);
            } else if let TemplateRoot::Dynamic { slot } = root {
                let index = slot.index();
                nodes_created += self.create_dynamic_node(
                    self.dynamic_values[index].node(),
                    mount,
                    index,
                    &mut state,
                );
            }
        }
        // Now that all descendants have been mounted and their raw mount slots
        // slots populated, snapshot ourselves into the mount. Using a
        // deep-clone here gives the snapshot its own per-vnode cells, so a
        // later `claim_mount` against a sibling subtree can't mutate
        // them out from under placement lookups that read this mount.
        state.dom.commit_mount(mount, self);
        nodes_created
    }
}

impl VNode {
    pub(crate) fn create_dynamic_node(
        &self,
        node: &DynamicNode,
        mount: MountId,
        idx: usize,
        state: &mut DiffState<'_, '_, '_>,
    ) -> usize {
        use DynamicNode::*;
        let parent = Some(MountRef { mount });
        match node {
            Component(c) => self.create_component_node(mount, idx, c, parent, state),
            Fragment(frag) => {
                state
                    .dom
                    .create_children(reborrow_writer(&mut state.to), frag, parent)
            }
            Text(text) => {
                // If we are diffing suspended nodes and are not outputting mutations, we can skip it
                if let Some(to) = reborrow_writer(&mut state.to) {
                    let id = state.dom.next_element_for_mount(mount);
                    state.dom.set_mounted_dynamic_text_node(mount, idx, id);
                    to.create_text(&text.value);
                    to.pop_id(id.element_id());
                    to.push_id(id.element_id());
                    1
                } else {
                    0
                }
            }
        }
    }

    fn load_dynamic_slots_for_static_node(
        &self,
        mount: MountId,
        path: TemplatePath,
        state: &mut DiffState<'_, '_, '_>,
    ) {
        // Reverse order lets earlier adjacent dynamic slots insert before
        // later siblings that have already materialized.
        for slot in dynamic_node_slots(self)
            .filter(|slot| slot.is_inside_static(path))
            .rev()
        {
            let dynamic_node_id = slot.index();
            let context = state.context();
            let site = insertion_site_for_slot(mount, slot, &[], state.dom, context);
            let runtime = state.dom.runtime.clone();
            let dom = &mut *state.dom;
            at_site(site, reborrow_writer(&mut state.to), runtime, |to| {
                let mut state = DiffState::new_with_context(dom, to, context);
                self.create_dynamic_node(
                    self.dynamic_values[dynamic_node_id].node(),
                    mount,
                    dynamic_node_id,
                    &mut state,
                )
            });
        }
    }

    fn write_attrs_for_static_node(
        &self,
        mount: MountId,
        path: TemplatePath,
        dom: &mut VirtualDom,
        to: &mut dyn WriteMutations,
    ) {
        for_each_dynamic_attr_group(self, |group| {
            if group.is_descendant_of_static(path) {
                self.write_attr_group(mount, &group, dom, to);
            }
        });
    }

    fn write_attr_group(
        &self,
        mount: MountId,
        group: &DynamicAttrGroup<'_>,
        dom: &mut VirtualDom,
        to: &mut dyn WriteMutations,
    ) {
        let id = self.assign_static_node_as_dynamic(mount, group, dom, to);
        for attribute_idx in group.ids() {
            for attr in self.dynamic_values[attribute_idx].attrs() {
                Self::write_attribute(attr, id, mount, dom, to);
            }
            // Store this even for empty dynamic attribute groups so fullstack
            // can later find where attributes may be inserted.
            dom.set_mounted_dyn_attr(mount, attribute_idx, id);
        }
    }

    /// We have some dynamic attributes attached to a some node
    ///
    /// That node needs to be loaded at runtime, so we need to give it an ID
    ///
    /// If the node in question is the root node, we just return the ID
    ///
    /// If the node is not on the stack, we create a new ID for it and assign it
    fn assign_static_node_as_dynamic(
        &self,
        mount: MountId,
        group: &DynamicAttrGroup<'_>,
        dom: &mut VirtualDom,
        to: &mut dyn WriteMutations,
    ) -> MountedElementId {
        let cursor = group.path();
        // This is just the root node. We already know it's id
        if group.is_root_level() {
            return dom.unchecked_mounted_root_node(mount, cursor.segment(0) as usize);
        }

        // The node is deeper in the template and we should create a new id for it
        let id = dom.next_element_for_mount(mount);

        let root_idx = cursor.segment(0) as usize;
        let root_id = dom.unchecked_mounted_root_node(mount, root_idx);
        with_consumed_id(to, root_id.element_id(), |to| {
            if let Some(indexes) = group.static_child_indexes() {
                for index in indexes {
                    to.child(index);
                }
            }
            to.pop_id(id.element_id());
        });

        id
    }

    fn load_template_root(
        &self,
        mount: MountId,
        root_idx: usize,
        root_op: usize,
        dom: &mut VirtualDom,
        to: &mut dyn WriteMutations,
    ) -> MountedElementId {
        let id = dom.next_element_for_mount(mount);
        dom.set_mounted_root_node(mount, root_idx, id);
        let target_id = dom.mount_target_id(mount);
        let template_id = match dom.cached_template_root(target_id, self.template, root_idx) {
            Some(id) => id,
            None => {
                let id = dom.allocate_template_root(target_id, self.template, root_idx);
                create_static_prototype(&self.template, root_op, to);
                to.pop_id(id.element_id());
                id
            }
        };
        to.push_id(template_id.element_id());
        WriteMutations::clone(to);
        to.pop_id(id.element_id());
        to.push_id(id.element_id());
        id
    }
}

fn create_static_prototype(template: &Template, op: usize, to: &mut dyn WriteMutations) -> usize {
    if let Some((tag, namespace)) = template.element_meta_at_op(op) {
        to.create_element(tag, namespace);

        let mut attr = template
            .element_children_start(op)
            .expect("element op must have a children start");
        let first_child = template
            .first_child_node_op(op)
            .expect("element op must have a first child op");
        while attr < first_child {
            if let Some((name, value, namespace)) = template.static_attr_at_op(attr) {
                let value = crate::AttributeValue::Text(value.to_string());
                to.set_attribute(name, namespace, &value);
                attr += template
                    .attr_op_len(attr)
                    .expect("static attr op must have a length");
            } else {
                attr += 1;
            }
        }

        let mut child = first_child;
        let end = template
            .element_end(op)
            .expect("element op must have an end op");
        let mut children = 0;
        while child < end {
            if template.is_static_node_op(child) {
                children += create_static_prototype(template, child, to);
            }
            child = template.next_sibling_op(child);
        }

        if children > 0 {
            to.append_children(children);
        }
        return 1;
    }

    if let Some(text) = template.static_text_at_op(op) {
        to.create_text(text);
        return 1;
    }

    unreachable!("static prototype root must start at a static node op")
}

fn current_scope_hidden_by_suspense(dom: &VirtualDom) -> bool {
    dom.runtime
        .try_current_scope_id()
        .and_then(|scope| dom.runtime.try_get_state(scope))
        .is_some_and(|scope| !scope.suspense_location().hidden_by().is_empty())
}

/// Look up the rendered root VNode for a component scope, for walking with
/// `find_element_in_roots` during placement.
///
/// The diff only resolves a component's rendered root once it has established
/// the component is live and rendered — placement resolution walks mounted
/// siblings, and `dynamic_node_first_element` runs under a `has_live_dom`
/// check — so a missing scope or unbuilt root is a bug, asserted here rather
/// than papered over with a silent `None`.
fn live_component_root(dom: &VirtualDom, scope_id: ScopeId) -> &VNode {
    dom.get_scope(scope_id)
        .expect("component scope must be live when resolving its rendered root")
        .root_node()
}

fn find_fragment_edge(
    children: &[VNode],
    dom: &VirtualDom,
    target_id: crate::RenderTargetId,
    edge: ElementEdge,
) -> Option<ElementId> {
    let find = |child: &VNode| child.find_element_in_roots(dom, target_id, edge);
    match edge {
        ElementEdge::First => children.iter().find_map(find),
        ElementEdge::Last => children.iter().rev().find_map(find),
    }
}
