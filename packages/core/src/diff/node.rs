use crate::{
    DynamicNode::*,
    MountedVNode, Template, VNode, VirtualDom, WriteMutations,
    arena::{ElementId, MountedElementId},
    diff::{
        CreatedVNode,
        context::{DiffFrame, DiffState},
        placement::{
            DomAnchor, ElementEdge, InsertionSite, at_site, create_at_site, find_root_dynamic_slot,
            insertion_site_at, insertion_site_for_slot,
        },
        template::{
            DynamicAttrGroup, DynamicNodeSlot, dynamic_node_slots,
            dynamic_node_slots_in_document_order, for_each_dynamic_attr_group,
        },
    },
    innerlude::{MountId, MountRef},
    mutations::{remove_id, with_consumed_id, with_id},
    nodes::DynamicNode,
    scopes::ScopeId,
};
use dioxus_core_template::TemplateAnchor;

impl MountedVNode<'_> {
    /// Diff this mounted vnode against `new`.
    ///
    /// Invariant: `self.mount()` is live and currently committed to `self.vnode()`. The returned
    /// mount is the committed mount for `new`; it may differ only when replacement required a new
    /// mount.
    pub(crate) fn diff_node(
        self,
        new: &VNode,
        dom: &mut VirtualDom,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> MountId {
        let mut state = DiffState::new(dom, to);
        DiffFrame::new(self.mount(), self.vnode(), new).diff_into(&mut state)
    }

    pub(crate) fn find_first_element(self, dom: &VirtualDom) -> Option<ElementId> {
        self.vnode().find_first_element(self.mount(), dom)
    }

    pub(crate) fn find_last_element(self, dom: &VirtualDom) -> Option<ElementId> {
        self.vnode().find_last_element(self.mount(), dom)
    }
}

impl<'a> DiffFrame<'a> {
    /// Diff one mounted vnode frame into the active diff state.
    ///
    /// Invariant: `self.mount` is live, `self.old` is the committed vnode for that mount, and
    /// `self.new` is the next vnode for the same logical position.
    pub(crate) fn diff_into(self, state: &mut DiffState<'_, '_, '_, '_>) -> MountId {
        let old = self.old;
        let new = self.new;

        let current_mount = self.mount;
        let mut state = state.reborrow_for_mount(current_mount);

        // If the templates are different, we need to replace the entire template.
        // `Template` equality includes the per-slot kind layout (see
        // `compute_hash`), so two vnodes that place an attribute where the other
        // places a node compare unequal here and take the full-replace path rather
        // than reaching the kind-assuming in-place diff below.
        if old.template != new.template {
            let parent = state.dom.mounted_render_parent(current_mount);
            let created = old.replace_inner(
                current_mount,
                std::slice::from_ref(new),
                parent,
                &mut state,
                true,
            );
            return created.mounts[0];
        }

        state.enter_context(current_mount, old, new);

        // If the templates are the same, we don't need to do anything, except copy over the mount information
        if old == new {
            state.dom.commit_mount(current_mount, new);
            return current_mount;
        }

        // If the templates are the same, we can diff the attributes and children
        // Start with the attributes
        // Since the attributes are only side effects, we can skip diffing them entirely if the node is suspended and we aren't outputting mutations
        if let Some(to) = state.to.as_deref_mut() {
            old.diff_attributes(current_mount, new, state.dom, to);
        }

        for slot in dynamic_node_slots_in_document_order(old) {
            let dyn_node_idx = slot.index();
            old.diff_dynamic_node(
                current_mount,
                slot,
                old.dynamic_values[dyn_node_idx].node(),
                new.dynamic_values[dyn_node_idx].node(),
                &mut state,
            );
        }
        state.dom.commit_mount(current_mount, new);
        current_mount
    }
}

impl VNode {
    /// Diff one dynamic node slot within a same-template vnode.
    ///
    /// Invariant: `slot.index()` points at the same dynamic value in `self`, `old_node`, and
    /// `new_node`; the mount table has a slot allocated for that index.
    fn diff_dynamic_node(
        &self,
        mount: MountId,
        slot: DynamicNodeSlot<'_>,
        old_node: &DynamicNode,
        new_node: &DynamicNode,
        state: &mut DiffState<'_, '_, '_, '_>,
    ) {
        let idx = slot.index();
        match (old_node, new_node) {
            (Text(old), Text(new)) => {
                // Diffing text is just a side effect, if we are diffing suspended nodes and are not outputting mutations, we can skip it
                if let Some(to) = state.to.as_deref_mut()
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
                self.diff_vcomponent(mount, idx, new, old, scope_id, state)
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
        state: &mut DiffState<'_, '_, '_, '_>,
    ) {
        let idx = slot.index();
        let old_has_live_dom = self.dynamic_node_has_live_dom(mount, idx, old, state.dom);
        if !old_has_live_dom {
            // The old slot has no renderer-owned nodes. Removal still needs to
            // clean mount records and component state, but it must not emit DOM
            // mutations.
            self.remove_dynamic_node(mount, state.dom, None, true, idx, old);
        }

        let live_first = if old_has_live_dom {
            let target_id = state.dom.current_render_target_id();
            self.dynamic_node_edge_element(
                mount,
                idx,
                old,
                state.dom,
                target_id,
                ElementEdge::First,
            )
        } else {
            None
        };
        let context = state.context();

        state.with_mounted_dynamic_node_slot_replaced(
            mount,
            idx,
            old_has_live_dom,
            |state| {
                if state.has_writer() {
                    let site = match live_first {
                        Some(first) => InsertionSite::AtAnchor(DomAnchor::Before(first)),
                        None => insertion_site_for_slot(mount, slot, state.dom, context),
                    };
                    let runtime = state.dom.runtime.clone();
                    let dom = &mut *state.dom;
                    let to = state.to.as_deref_mut().expect("writer checked");
                    at_site(site, to, runtime, |to| {
                        let mut state = DiffState::new_with_context(dom, Some(to), context);
                        self.create_dynamic_node(new, mount, idx, &mut state)
                    });
                } else {
                    self.create_dynamic_node(new, mount, idx, state);
                }
            },
            |state| {
                self.remove_dynamic_node(mount, state.dom, state.to.as_deref_mut(), true, idx, old);
            },
        );
    }

    /// Diff two fragments at a dynamic slot.
    ///
    /// Invariant: `old` and the committed fragment child mount range have exactly the same length.
    /// Empty fragments own no DOM and are represented by an empty mounted slot.
    fn diff_fragment(
        &self,
        mount: MountId,
        slot: DynamicNodeSlot<'_>,
        old: &[VNode],
        new: &[VNode],
        state: &mut DiffState<'_, '_, '_, '_>,
    ) {
        let parent = Some(MountRef { mount });
        let old_mounts = state
            .dom
            .mounted_fragment_children_exact(mount, slot.index(), old.len());
        match (old.is_empty(), new.is_empty()) {
            (true, true) => {
                state
                    .dom
                    .clear_mounted_fragment_children(mount, slot.index());
            }
            (true, false) => {
                // Empty → non-empty: visible diffs stage new content at the
                // slot insertion site. Hidden/no-writer diffs only materialize
                // mount state, so there is no renderer placement to resolve.
                let created = state.create_children_at_site(new, parent, |state| {
                    insertion_site_for_slot(mount, slot, state.dom, state.context())
                });
                state
                    .dom
                    .set_mounted_fragment_children_vec(mount, slot.index(), created.mounts);
            }
            (false, true) => {
                state
                    .dom
                    .remove_nodes(state.to.as_deref_mut(), old, &old_mounts);
                state
                    .dom
                    .clear_mounted_fragment_children(mount, slot.index());
            }
            (false, false) => {
                let new_mounts = state.diff_non_empty_fragment(old, &old_mounts, new, parent);
                state
                    .dom
                    .set_mounted_fragment_children_vec(mount, slot.index(), new_mounts);
            }
        }
    }

    pub(crate) fn find_first_element(&self, mount: MountId, dom: &VirtualDom) -> Option<ElementId> {
        self.find_element_in_roots(
            mount,
            dom,
            dom.current_render_target_id(),
            ElementEdge::First,
        )
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
        debug_assert!(
            root_idx < dom.mounted_root_count(mount),
            "mounted root count must match the vnode template"
        );
        dom.mounted_root_node(mount, root_idx)
            .filter(|id| dom.element_exists_in_target(target_id, *id))
            .map(MountedElementId::element_id)
    }

    pub(crate) fn find_last_element(&self, mount: MountId, dom: &VirtualDom) -> Option<ElementId> {
        self.find_element_in_roots(
            mount,
            dom,
            dom.current_render_target_id(),
            ElementEdge::Last,
        )
    }

    fn has_live_dom(&self, mount: MountId, dom: &VirtualDom) -> bool {
        debug_assert_eq!(
            self.template.root_count(),
            dom.mounted_root_count(mount),
            "mounted root count must match the vnode template"
        );
        debug_assert_eq!(
            self.template.dynamic_value_count(),
            dom.mounted_dyn_node_count(mount),
            "slot count"
        );

        if (0..self.template.root_count()).any(|root_idx| {
            dom.mounted_root_node(mount, root_idx)
                .is_some_and(|id| dom.element_exists_for_mount(mount, id))
        }) {
            return true;
        }

        dynamic_node_slots(self).any(|slot| {
            if !slot.is_root_level() {
                return false;
            }

            let idx = slot.index();
            self.dynamic_node_has_live_dom(mount, idx, self.dynamic_values[idx].node(), dom)
        })
    }

    fn find_element_in_roots(
        &self,
        mount: MountId,
        dom: &VirtualDom,
        target_id: crate::RenderTargetId,
        edge: ElementEdge,
    ) -> Option<ElementId> {
        edge.find_map(self.template.root_count(), |root_idx| {
            let dynamic =
                || self.find_root_dynamic_at_cursor(root_idx, mount, target_id, dom, edge);
            let static_root =
                || self.find_element_at_root_in_target(root_idx, mount, target_id, dom);
            match edge {
                ElementEdge::First => dynamic().or_else(static_root),
                ElementEdge::Last => static_root().or_else(dynamic),
            }
        })
    }

    fn find_root_dynamic_at_cursor(
        &self,
        cursor_idx: usize,
        mount: MountId,
        target_id: crate::RenderTargetId,
        dom: &VirtualDom,
        edge: ElementEdge,
    ) -> Option<ElementId> {
        find_root_dynamic_slot(self, cursor_idx, edge, |slot| {
            let idx = slot.index();
            self.dynamic_node_edge_element(
                mount,
                idx,
                self.dynamic_values[idx].node(),
                dom,
                target_id,
                edge,
            )
        })
    }

    /// Replace this node with `right`, reusing an already allocated mount for
    /// the replacement.
    pub(crate) fn replace_with_existing_mount(
        &self,
        mount: MountId,
        right: &VNode,
        right_mount: MountId,
        parent: Option<MountRef>,
        dom: &mut VirtualDom,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> CreatedVNode {
        let mut state = DiffState::new(dom, to);
        let nodes = if state.has_writer() {
            // The replacement mount is already allocated and must not anchor the
            // insertion against itself; mark it stale for this lookup only.
            state.dom.runtime.mark_placement_stale(right_mount);
            let site = insertion_site_at(
                ElementEdge::First,
                MountedVNode::new(self, mount),
                state.dom,
                state.context(),
            );
            state.dom.runtime.unmark_placement_stale(right_mount);
            let runtime = state.dom.runtime.clone();
            let dom = &mut *state.dom;
            let to = state.to.as_deref_mut().expect("writer checked");
            at_site(site, to, runtime, |to| {
                right
                    .recreate_with_mount(dom, right_mount, parent, parent, Some(to))
                    .nodes
            })
        } else {
            right
                .recreate_with_mount(
                    state.dom,
                    right_mount,
                    parent,
                    parent,
                    state.to.as_deref_mut(),
                )
                .nodes
        };

        self.remove_node_inner(mount, state.dom, state.to.as_deref_mut(), true);

        CreatedVNode {
            nodes,
            mount: right_mount,
        }
    }

    /// Replace this node with new children, but *don't destroy* the old node's component state
    ///
    /// This is useful for moving a node from the rendered nodes into a suspended node
    pub(crate) fn move_node_to_background(
        &self,
        mount: MountId,
        right: &[VNode],
        parent: Option<MountRef>,
        dom: &mut VirtualDom,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> crate::diff::CreatedNodes {
        let mut state = DiffState::new(dom, to);
        self.replace_inner(mount, right, parent, &mut state, false)
    }

    pub(crate) fn replace_inner(
        &self,
        mount: MountId,
        right: &[VNode],
        parent: Option<MountRef>,
        state: &mut DiffState<'_, '_, '_, '_>,
        destroy_component_state: bool,
    ) -> crate::diff::CreatedNodes {
        // When the old subtree has no live DOM and the boundary is hidden, we
        // still materialize mount/component state for the new subtree, but no
        // renderer insertion site exists or is needed.
        let suppress_mutations =
            self.should_suppress_mutations(mount, state.dom, destroy_component_state);
        let context = state.context();
        let write_local_mutations = !suppress_mutations && state.has_writer();
        let created = if write_local_mutations {
            let site = insertion_site_at(
                ElementEdge::First,
                MountedVNode::new(self, mount),
                state.dom,
                context,
            );
            let to = state.to.as_deref_mut().expect("writer checked");
            create_at_site(right, parent, site, state.dom, to)
        } else {
            let to = if suppress_mutations {
                None
            } else {
                state.to.as_deref_mut()
            };
            state
                .dom
                .create_children_with_parents(to, right, parent, parent)
        };
        let to_for_remove = if suppress_mutations {
            None
        } else {
            state.to.as_deref_mut()
        };
        self.remove_node_inner(mount, state.dom, to_for_remove, destroy_component_state);
        created
    }

    /// True when we may skip emitting renderer mutations for a replace because
    /// the old subtree has no live DOM and we're operating inside a suspended
    /// boundary (or have no `WriteMutations` sink at all).
    fn should_suppress_mutations(
        &self,
        mount: MountId,
        dom: &VirtualDom,
        destroy_component_state: bool,
    ) -> bool {
        if !destroy_component_state {
            return false;
        }
        if self.has_live_dom(mount, dom) {
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

    /// Remove a node from the DOM and destroy component state.
    ///
    /// Invariant: `mount` is live and committed to `self` when removal begins.
    pub(crate) fn remove_node(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) {
        self.remove_node_inner(mount, dom, to, true)
    }

    /// Remove a node, optionally preserving component state.
    ///
    /// Invariant: preserving component state is used only when suspense moves an already-rendered
    /// branch out of the foreground DOM; mount ownership remains with the retained branch.
    pub(crate) fn remove_node_inner(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        to: Option<&mut (dyn WriteMutations + '_)>,
        destroy_component_state: bool,
    ) {
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
            dom.remove_mount(mount);
        }
    }

    fn reclaim_roots(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        mut to: Option<&mut (dyn WriteMutations + '_)>,
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
                    to.as_deref_mut(),
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
            if let Some(to) = to.as_deref_mut() {
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
        mut to: Option<&mut (dyn WriteMutations + '_)>,
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
                dom.clear_mounted_dynamic_node_slot(mount, idx);
            }
            Fragment(nodes) => {
                let mounts = dom.mounted_fragment_children_exact(mount, idx, nodes.len());
                for (node, child_mount) in nodes.iter().zip(mounts) {
                    node.remove_node_inner(
                        child_mount,
                        dom,
                        to.as_deref_mut(),
                        destroy_component_state,
                    );
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
                    .and_then(|scope| scope.try_mounted_root_node())
                    .is_some_and(|node| node.vnode().has_live_dom(node.mount(), dom))
            }
            Text(_) => dom
                .mounted_dynamic_text_node(mount, idx)
                .is_some_and(|id| dom.element_exists_for_mount(mount, id)),
            Fragment(nodes) => {
                let mounts = dom.mounted_fragment_children_exact(mount, idx, nodes.len());
                nodes
                    .iter()
                    .zip(mounts)
                    .any(|(node, mount)| node.has_live_dom(mount, dom))
            }
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
                let root = live_component_root(dom, scope_id);
                root.find_element_in_roots(root.mount(), dom, target_id, edge)
            }
            Text(_) if dom.mount_target_id(mount) == target_id => dom
                .mounted_dynamic_text_node(mount, idx)
                .filter(|id| dom.element_exists_in_target(target_id, *id))
                .map(MountedElementId::element_id),
            Text(_) => None,
            Fragment(nodes) => {
                let mounts = dom.mounted_fragment_children_exact(mount, idx, nodes.len());
                edge.find_map(nodes.len(), |idx| {
                    nodes[idx].find_element_in_roots(mounts[idx], dom, target_id, edge)
                })
            }
        }
    }

    pub(super) fn reclaim_attributes(&self, mount: MountId, dom: &mut VirtualDom) {
        let mut next_id = None;
        for_each_dynamic_attr_group(self, |group| {
            // We clean up the roots in the next step, so don't worry about them here
            if group.static_path().len() == 1 {
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

    /// Create this vnode under explicit render/logical parents.
    ///
    /// Invariant: when `to` is `Some`, the new mount is foreground-renderable and every static
    /// template root receives a mounted root id.
    pub(crate) fn create_with_parents(
        &self,
        dom: &mut VirtualDom,
        render_parent: Option<MountRef>,
        logical_parent: Option<MountRef>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> CreatedVNode {
        self.create_with_optional_mount(dom, None, render_parent, logical_parent, to)
    }

    /// Recreate this vnode using an existing mount id.
    ///
    /// Invariant: the existing mount belongs to the same logical component/branch being promoted or
    /// rerendered; `commit_mount` will replace its vnode only after roots/dynamic slots are loaded.
    pub(crate) fn recreate_with_mount(
        &self,
        dom: &mut VirtualDom,
        mount: MountId,
        render_parent: Option<MountRef>,
        logical_parent: Option<MountRef>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> CreatedVNode {
        self.create_with_optional_mount(dom, Some(mount), render_parent, logical_parent, to)
    }

    fn create_with_optional_mount(
        &self,
        dom: &mut VirtualDom,
        existing_mount: Option<MountId>,
        render_parent: Option<MountRef>,
        logical_parent: Option<MountRef>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> CreatedVNode {
        let mut state = DiffState::new(dom, to);
        let target_id = state.dom.current_render_target_id();

        let mount = if let Some(mount) = existing_mount {
            state
                .dom
                .reuse_mount(mount, render_parent, logical_parent, target_id);
            mount
        } else {
            state
                .dom
                .create_mount(self, render_parent, logical_parent, target_id)
        };
        debug_assert!(
            state.to.is_none() || state.dom.mount_should_render(mount),
            "background mounts must be created without renderer writes"
        );

        let reuse_existing_mounts = existing_mount.is_some();
        let nodes_created =
            self.materialize_template_roots(mount, &mut state, reuse_existing_mounts);
        self.fill_dynamic_values(mount, &mut state, reuse_existing_mounts);

        state.dom.commit_mount(mount, self);
        CreatedVNode {
            nodes: nodes_created,
            mount,
        }
    }
}

impl VNode {
    fn materialize_template_roots(
        &self,
        mount: MountId,
        state: &mut DiffState<'_, '_, '_, '_>,
        reuse_existing_mounts: bool,
    ) -> usize {
        let mut nodes_created = 0;

        for (root_idx, static_op, dynamic_anchor) in self.template.root_slots() {
            if let Some(anchor) = dynamic_anchor {
                for index in self.dynamic_node_indices_for_anchor(anchor) {
                    nodes_created += self.create_dynamic_node_inner(
                        self.dynamic_values[index].node(),
                        mount,
                        index,
                        state,
                        reuse_existing_mounts,
                    );
                }
                continue;
            }

            let root_op = static_op.expect("root slot must be static or dynamic");

            if let Some(to) = state.to.as_deref_mut() {
                self.load_template_root(mount, root_idx, root_op, state.dom, to);
                nodes_created += 1;
            }
        }

        nodes_created
    }

    fn fill_dynamic_values(
        &self,
        mount: MountId,
        state: &mut DiffState<'_, '_, '_, '_>,
        reuse_existing_mounts: bool,
    ) {
        for anchor in self.template.anchors() {
            let group = DynamicAttrGroup::new(self, anchor);
            if let Some(to) = state.to.as_deref_mut() {
                self.write_attr_group(mount, &group, state.dom, to);
            }

            let has_dynamic_nodes = anchor
                .values()
                .any(|idx| self.dynamic_values[idx].as_node().is_some());
            if has_dynamic_nodes && anchor.parent_element_op_index().is_some() {
                self.load_dynamic_anchor(mount, anchor, state, reuse_existing_mounts);
            }
        }
    }

    /// Create one dynamic node value in an already allocated mount slot.
    ///
    /// Invariant: `idx` is allocated in `mount` and matches `node`'s position in `self`.
    pub(crate) fn create_dynamic_node(
        &self,
        node: &DynamicNode,
        mount: MountId,
        idx: usize,
        state: &mut DiffState<'_, '_, '_, '_>,
    ) -> usize {
        self.create_dynamic_node_inner(node, mount, idx, state, false)
    }

    fn create_dynamic_node_inner(
        &self,
        node: &DynamicNode,
        mount: MountId,
        idx: usize,
        state: &mut DiffState<'_, '_, '_, '_>,
        reuse_existing_mounts: bool,
    ) -> usize {
        use DynamicNode::*;
        let parent = Some(MountRef { mount });
        match node {
            Component(c) => self.create_component_node(mount, idx, c, state),
            Fragment(frag) => {
                if reuse_existing_mounts {
                    let mounts = state
                        .dom
                        .mounted_fragment_children_exact(mount, idx, frag.len());
                    let mut nodes = 0;
                    for (child, child_mount) in frag.iter().zip(mounts.iter().copied()) {
                        let created = child.recreate_with_mount(
                            state.dom,
                            child_mount,
                            parent,
                            parent,
                            state.to.as_deref_mut(),
                        );
                        nodes += created.nodes;
                    }
                    state
                        .dom
                        .set_mounted_fragment_children_vec(mount, idx, mounts);
                    return nodes;
                }

                let created = state.dom.create_children_with_parents(
                    state.to.as_deref_mut(),
                    frag,
                    parent,
                    parent,
                );
                let nodes = created.nodes;
                state
                    .dom
                    .set_mounted_fragment_children_vec(mount, idx, created.mounts);
                nodes
            }
            Text(text) => {
                // If we are diffing suspended nodes and are not outputting mutations, we can skip it
                if let Some(to) = state.to.as_deref_mut() {
                    let target_id = state.dom.current_render_target_id();
                    let id = state.dom.next_element_in_target(target_id);
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

    fn load_dynamic_anchor(
        &self,
        mount: MountId,
        anchor: &TemplateAnchor,
        state: &mut DiffState<'_, '_, '_, '_>,
        reuse_existing_mounts: bool,
    ) {
        let first_node_id = anchor
            .values()
            .find(|&idx| self.dynamic_values[idx].as_node().is_some())
            .expect("node anchor");
        let slot = DynamicNodeSlot::new(&self.template, anchor, first_node_id);
        if !state.has_writer() {
            for dynamic_node_id in anchor
                .values()
                .filter(|&idx| self.dynamic_values[idx].as_node().is_some())
            {
                self.create_dynamic_node_inner(
                    self.dynamic_values[dynamic_node_id].node(),
                    mount,
                    dynamic_node_id,
                    state,
                    reuse_existing_mounts,
                );
            }
            return;
        }

        let context = state.context();
        let site = self.template_slot_insertion_site(mount, slot, state.dom);
        let runtime = state.dom.runtime.clone();
        let dom = &mut *state.dom;
        let to = state.to.as_deref_mut().expect("writer checked");
        at_site(site, to, runtime, |to| {
            let mut state = DiffState::new_with_context(dom, Some(to), context);
            anchor
                .values()
                .filter(|&idx| self.dynamic_values[idx].as_node().is_some())
                .map(|dynamic_node_id| {
                    self.create_dynamic_node_inner(
                        self.dynamic_values[dynamic_node_id].node(),
                        mount,
                        dynamic_node_id,
                        &mut state,
                        reuse_existing_mounts,
                    )
                })
                .sum()
        });
    }

    fn template_slot_insertion_site(
        &self,
        mount: MountId,
        slot: DynamicNodeSlot<'_>,
        dom: &VirtualDom,
    ) -> InsertionSite {
        debug_assert!(
            !slot.is_root_level(),
            "non-root dynamic anchors must have an enclosing template root"
        );
        let root_id = dom.unchecked_mounted_root_node(mount, slot.root_index());
        InsertionSite::Slot {
            parent: root_id.element_id(),
            placement: slot.placement(),
        }
    }

    fn write_attr_group(
        &self,
        mount: MountId,
        group: &DynamicAttrGroup<'_>,
        dom: &mut VirtualDom,
        to: &mut dyn WriteMutations,
    ) {
        // A pure dynamic-node anchor (e.g. a root-level node slot) decorates no
        // static element, so it has no attributes to write and no static path to
        // resolve. Skip it before `assign_static_node_as_dynamic` tries to.
        if group.ids().next().is_none() {
            return;
        }
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
        let cursor = group.static_path();
        let root_idx = group.root_index();
        // This is just the root node. We already know it's id
        if group.is_root_level() {
            return dom.unchecked_mounted_root_node(mount, root_idx);
        }

        // The node is deeper in the template and we should create a new id for it
        let target_id = dom.current_render_target_id();
        let id = dom.next_element_in_target(target_id);

        let root_id = dom.unchecked_mounted_root_node(mount, root_idx);
        with_consumed_id(to, root_id.element_id(), |to| {
            for depth in 1..cursor.len() {
                to.child(cursor.segment(depth) as usize);
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
        let target_id = dom.current_render_target_id();
        let id = dom.next_element_in_target(target_id);
        dom.set_mounted_root_node(mount, root_idx, id);

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

        let mut attr = template.element_children_start(op).expect("bad element");
        let first_child = template.first_child_node_op(op).expect("bad element");
        while attr < first_child {
            let (name, value, namespace) =
                template.static_attr_at_op(attr).expect("bad static attr");
            let value = crate::AttributeValue::Text(value.to_string());
            to.set_attribute(name, namespace, &value);
            attr += template.attr_op_len(attr).expect("bad static attr");
        }

        let mut child = first_child;
        let end = template.element_end(op).expect("bad element");
        let mut children = 0;
        while child < end {
            children += create_static_prototype(template, child, to);
            child = template.next_sibling_op(child);
        }

        if children > 0 {
            to.append_children(children);
        }
        return 1;
    }

    let text = template.static_text_at_op(op).expect("bad static root");
    to.create_text(text);
    1
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
/// Callers resolve a component's rendered root only after establishing the
/// component is live and rendered (placement resolution walks mounted siblings,
/// and dynamic replacement asks for a component edge only after a live-DOM
/// check), so this panics if the scope or its root is missing.
fn live_component_root(dom: &VirtualDom, scope_id: ScopeId) -> MountedVNode<'_> {
    dom.get_scope(scope_id)
        .expect("component scope")
        .try_mounted_root_node()
        .expect("component root")
}
