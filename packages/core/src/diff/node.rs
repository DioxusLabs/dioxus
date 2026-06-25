use crate::{
    DynamicNode::*,
    MountedVNode, VNode, VNodeChild, VirtualDom, WriteMutations,
    arena::{ElementId, MountedElementId},
    diff::{
        CreatedVNode,
        attributes::{AttrDiffTarget, AttributeDiffScratch},
        context::{DiffFrame, DiffState},
        placement::{
            ElementEdge, InsertionSite, at_site, create_at_site, insertion_site_at,
            insertion_site_for_slot,
        },
        template::{DynamicAnchor, DynamicNodeSlot},
    },
    innerlude::MountId,
    mutations::{remove_id, with_consumed_id, with_id},
    nodes::{DynamicNode, VText},
    scopes::ScopeId,
};
use dioxus_core_template::{StaticTemplateNode, TemplateAnchor, TemplatePath};

/// How a dynamic-slot edge scan ([`VNode::dynamic_node_edge_element`]) reads mount state.
///
/// `find_first`/`find_last` walk the *live* render output of a subtree and trust every mount
/// ([`EdgeScan::live`]). Placement sibling scans ([`EdgeScan::placement`]) read the committed
/// component mount view so placement queries only observe coherent committed state.
#[derive(Clone, Copy)]
pub(super) struct EdgeScan {
    target_id: crate::RenderTargetId,
    committed_component_view: bool,
}

impl EdgeScan {
    /// Walk the live render output of a subtree in `target_id`, trusting every mount.
    pub(super) fn live(target_id: crate::RenderTargetId) -> Self {
        Self {
            target_id,
            committed_component_view: false,
        }
    }

    /// Scan a sibling slot for a placement anchor in the current render target using the committed
    /// component root view.
    pub(super) fn placement(dom: &VirtualDom) -> Self {
        Self {
            target_id: dom.current_render_target_id(),
            committed_component_view: true,
        }
    }
}

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
        // `Template` equality includes anchor node/attribute ranges, so two vnodes
        // with incompatible dynamic slot layouts compare unequal here and take the
        // full-replace path.
        if old.template() != new.template() {
            let parent = state.dom.mounted_render_parent(current_mount);
            let created = old.replace_inner(current_mount, new, parent, &mut state, true);
            return created.mount;
        }

        state.enter_context(current_mount, old, new);

        // If the templates are the same, we don't need to do anything, except copy over the mount information
        if old == new {
            state.dom.commit_mount(current_mount, new);
            return current_mount;
        }

        // If the templates are the same, we can diff the attributes and children
        let mut scratch = AttributeDiffScratch::default();
        for anchor in old.dynamic_anchors() {
            if anchor.attrs().len() > 0 {
                // Since the attributes are only side effects, we can skip diffing them entirely if the node is suspended and we aren't outputting mutations
                if let Some(to) = state.to.as_deref_mut() {
                    let attribute_id = state
                        .dom
                        .unchecked_mounted_anchor_node(current_mount, anchor.anchor_index());
                    let new_anchor = new.dynamic_anchor(anchor.anchor_index());
                    old.diff_attribute_list(
                        anchor,
                        new_anchor,
                        AttrDiffTarget::new(attribute_id, current_mount),
                        &mut scratch,
                        state.dom,
                        to,
                    );
                }
            }

            for slot in anchor.nodes() {
                old.diff_dynamic_node(current_mount, slot, new, &mut state);
            }
        }
        state.dom.commit_mount(current_mount, new);
        current_mount
    }
}

impl VNode {
    /// Diff one dynamic node slot within a same-template vnode.
    ///
    /// Invariant: `slot.index()` points at the same dynamic node in `self` and `new`; the mount
    /// table has a slot allocated for that index.
    fn diff_dynamic_node(
        &self,
        mount: MountId,
        slot: DynamicNodeSlot<'_>,
        new: &VNode,
        state: &mut DiffState<'_, '_, '_, '_>,
    ) {
        let idx = slot.index();
        let old_node = &self.dynamic_node_values()[idx];
        let new_node = &new.dynamic_node_values()[idx];
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
                self.diff_vcomponent(mount, slot, new, old, scope_id, state)
            }
            _ => self.replace_dynamic_node_at_slot(mount, slot, new, state),
        };
    }

    fn replace_dynamic_node_at_slot(
        &self,
        mount: MountId,
        slot: DynamicNodeSlot<'_>,
        new: &VNode,
        state: &mut DiffState<'_, '_, '_, '_>,
    ) {
        let idx = slot.index();
        // The old slot's first live element (if any) is both the "is there live
        // DOM to remove" signal and the anchor the replacement inserts before.
        let target_id = state.dom.current_render_target_id();
        let live_first = self.dynamic_node_edge_element(
            mount,
            idx,
            state.dom,
            EdgeScan::live(target_id),
            ElementEdge::First,
        );
        let old_has_live_dom = live_first.is_some();
        let context = state.context();
        let site = state.has_writer().then(|| {
            live_first
                .map(InsertionSite::before)
                .unwrap_or_else(|| insertion_site_for_slot(mount, slot, state.dom, context))
        });

        if !old_has_live_dom {
            // The old slot has no nodes in the current target. It may still own component nodes
            // routed to another target (for example a portal), so let those removals write through
            // the target router while replacement placement stays anchored by the empty slot.
            let to = if self.dynamic_component_targets_other_render_target(mount, idx, state.dom) {
                state.to.as_deref_mut()
            } else {
                None
            };
            self.remove_dynamic_node(mount, state.dom, to, true, idx);
        }

        let create_new = |state: &mut DiffState<'_, '_, '_, '_>| {
            if let Some(site) = site {
                let runtime = state.dom.runtime.clone();
                let dom = &mut *state.dom;
                let to = state.to.as_deref_mut().expect("writer checked");
                at_site(site, to, runtime, |to| {
                    let mut state = DiffState::new_with_context(dom, Some(to), context);
                    new.create_dynamic_node(mount, idx, &mut state)
                });
            } else {
                new.create_dynamic_node(mount, idx, state);
            }
        };

        if old_has_live_dom {
            state.replace_live_mounted_dynamic_node_slot(mount, idx, create_new, |state| {
                self.remove_dynamic_node(mount, state.dom, state.to.as_deref_mut(), true, idx);
            });
        } else {
            state.replace_mounted_dynamic_node_slot(mount, idx, create_new, |_| {});
        }
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
        let parent = Some(mount);
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
                let context = state.context();
                let site = state
                    .has_writer()
                    .then(|| insertion_site_for_slot(mount, slot, state.dom, context));
                let children =
                    state
                        .dom
                        .begin_mounted_fragment_children(mount, slot.index(), new.len());
                state.create_children_at_site(
                    new,
                    parent,
                    |_| site.expect("visible fragment creation requires an insertion site"),
                    children,
                );
                state.dom.commit_mounted_fragment_children(children);
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
                let context = state.context();
                let fallback_site = state
                    .has_writer()
                    .then(|| insertion_site_for_slot(mount, slot, state.dom, context));
                let children =
                    state
                        .dom
                        .begin_mounted_fragment_children(mount, slot.index(), new.len());
                state.diff_non_empty_fragment(
                    old,
                    &old_mounts,
                    new,
                    parent,
                    children,
                    fallback_site,
                );
                state.dom.commit_mounted_fragment_children(children);
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

    fn find_static_anchor_in_target(
        &self,
        anchor_idx: usize,
        mount: MountId,
        target_id: crate::RenderTargetId,
        dom: &VirtualDom,
    ) -> Option<ElementId> {
        if dom.mount_target_id(mount) != target_id {
            return None;
        }
        dom.mounted_anchor_node(mount, anchor_idx)
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

    pub(super) fn find_element_in_roots(
        &self,
        mount: MountId,
        dom: &VirtualDom,
        target_id: crate::RenderTargetId,
        edge: ElementEdge,
    ) -> Option<ElementId> {
        match edge {
            ElementEdge::First => self
                .children()
                .find_map(|child| self.root_child_edge_element(child, mount, target_id, dom, edge)),
            ElementEdge::Last => {
                let mut found = None;
                for child in self.children() {
                    if let Some(id) =
                        self.root_child_edge_element(child, mount, target_id, dom, edge)
                    {
                        found = Some(id);
                    }
                }
                found
            }
        }
    }

    fn root_child_edge_element(
        &self,
        child: VNodeChild<'_>,
        mount: MountId,
        target_id: crate::RenderTargetId,
        dom: &VirtualDom,
        edge: ElementEdge,
    ) -> Option<ElementId> {
        match child {
            VNodeChild::Dynamic(anchor) => {
                self.dynamic_anchor_edge_element(anchor, mount, dom, target_id, edge)
            }
            VNodeChild::Element(element) => self.find_static_anchor_in_target(
                element.anchor_index().expect("root element"),
                mount,
                target_id,
                dom,
            ),
            VNodeChild::Text(text) => self.find_static_anchor_in_target(
                text.anchor_index().expect("root text"),
                mount,
                target_id,
                dom,
            ),
        }
    }

    fn dynamic_anchor_edge_element(
        &self,
        anchor: DynamicAnchor<'_>,
        mount: MountId,
        dom: &VirtualDom,
        target_id: crate::RenderTargetId,
        edge: ElementEdge,
    ) -> Option<ElementId> {
        let scan = EdgeScan::live(target_id);
        match edge {
            ElementEdge::First => anchor.nodes().find_map(|slot| {
                self.dynamic_node_edge_element(mount, slot.index(), dom, scan, edge)
            }),
            ElementEdge::Last => anchor.nodes().rev().find_map(|slot| {
                self.dynamic_node_edge_element(mount, slot.index(), dom, scan, edge)
            }),
        }
    }

    /// Replace this node with new children, but *don't destroy* the old node's component state
    ///
    /// This is useful for moving a node from the rendered nodes into a suspended node
    pub(crate) fn move_node_to_background(
        &self,
        mount: MountId,
        new: &VNode,
        parent: Option<MountId>,
        dom: &mut VirtualDom,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> CreatedVNode {
        let mut state = DiffState::new(dom, to);
        self.replace_inner(mount, new, parent, &mut state, false)
    }

    pub(crate) fn replace_inner(
        &self,
        mount: MountId,
        new: &VNode,
        parent: Option<MountId>,
        state: &mut DiffState<'_, '_, '_, '_>,
        destroy_component_state: bool,
    ) -> CreatedVNode {
        let live_first = self.find_first_element(mount, state.dom);
        let context = state.context();
        let created = if state.has_writer() {
            let site = live_first.map(InsertionSite::before).unwrap_or_else(|| {
                insertion_site_at(MountedVNode::new(self, mount), state.dom, context)
            });
            let to = state.to.as_deref_mut().expect("writer checked");
            create_at_site(new, parent, site, state.dom, to)
        } else {
            new.create_mounted(state.dom, parent, parent, state.to.as_deref_mut())
        };
        self.remove_node_inner(
            mount,
            state.dom,
            state.to.as_deref_mut(),
            destroy_component_state,
        );
        created
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
    /// Invariant: when `destroy_component_state` is false, mount ownership remains with the
    /// retained suspense branch.
    pub(crate) fn remove_node_inner(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        to: Option<&mut (dyn WriteMutations + '_)>,
        destroy_component_state: bool,
    ) {
        let (root_ids, root_anchor_indices) =
            self.prepare_node_removal(mount, dom, destroy_component_state);

        // Clean up the roots, assuming we need to generate mutations for these
        // This is done last in order to preserve Node ID reclaim order (reclaim in reverse order of claim)
        self.reclaim_roots(
            mount,
            dom,
            to,
            destroy_component_state,
            &root_ids,
            &root_anchor_indices,
        );

        if destroy_component_state {
            dom.remove_mount(mount);
        }
    }

    fn prepare_node_removal(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        destroy_component_state: bool,
    ) -> (Vec<usize>, Vec<usize>) {
        let mut reclaimed = Vec::new();
        let mut root_ids = Vec::new();
        let mut root_anchor_indices = Vec::new();

        for (anchor_idx, anchor) in self.template().anchors().iter().enumerate() {
            if let Some(path) = anchor_static_target(anchor) {
                if path.is_root() {
                    root_anchor_indices.push(anchor_idx);
                } else if let Some(id) = dom.mounted_anchor_node(mount, anchor_idx) {
                    if !reclaimed.contains(&id) {
                        dom.reclaim_for_mount(mount, id);
                        reclaimed.push(id);
                    }
                    dom.clear_mounted_anchor_node(mount, anchor_idx);
                }
            }

            // Root-level node anchors own renderer mutations and reclaim after nested ones; this is
            // a static property of the anchor's slot target.
            let root_level = anchor.parent_element_op_index().is_none();
            for idx in anchor.nodes() {
                if root_level {
                    root_ids.push(idx);
                } else {
                    self.remove_dynamic_node(mount, dom, None, destroy_component_state, idx);
                }
            }
        }

        (root_ids, root_anchor_indices)
    }

    fn reclaim_roots(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        mut to: Option<&mut (dyn WriteMutations + '_)>,
        destroy_component_state: bool,
        root_ids: &[usize],
        root_anchor_indices: &[usize],
    ) {
        for &id in root_ids {
            let dynamic_node = &self.dynamic_node_values()[id];
            // Empty Fragments contribute no DOM and have nothing to reclaim
            // via the renderer - skip them entirely.
            if matches!(dynamic_node, DynamicNode::Fragment(nodes) if nodes.is_empty()) {
                continue;
            }
            self.remove_dynamic_node(mount, dom, to.as_deref_mut(), destroy_component_state, id);
        }

        let mut removed_roots = Vec::new();
        for &anchor_idx in root_anchor_indices {
            if let Some(id) = dom.mounted_anchor_node(mount, anchor_idx)
                && !removed_roots.contains(&id)
            {
                if let Some(to) = to.as_deref_mut() {
                    remove_id(to, id.element_id());
                }
                dom.reclaim_for_mount(mount, id);
                removed_roots.push(id);
            }
            dom.clear_mounted_anchor_node(mount, anchor_idx);
        }
    }

    fn remove_dynamic_node(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        mut to: Option<&mut (dyn WriteMutations + '_)>,
        destroy_component_state: bool,
        idx: usize,
    ) {
        let node = &self.dynamic_node_values()[idx];
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

    /// The live DOM element on `edge` contributed by one dynamic node slot, or `None` when the slot
    /// contributes nothing in `scan.target_id`: an empty fragment/component, a node in another
    /// render target, or a component whose committed root has no live edge.
    pub(super) fn dynamic_node_edge_element(
        &self,
        mount: MountId,
        idx: usize,
        dom: &VirtualDom,
        scan: EdgeScan,
        edge: ElementEdge,
    ) -> Option<ElementId> {
        let EdgeScan {
            target_id,
            committed_component_view,
        } = scan;
        match &self.dynamic_node_values()[idx] {
            Text(_) if dom.mount_target_id(mount) == target_id => dom
                .mounted_dynamic_text_node(mount, idx)
                .map(MountedElementId::element_id),
            Text(_) => None,
            Fragment(nodes) => dom
                .try_with_mounted_fragment_children(mount, idx, nodes.len(), |child_mounts| {
                    edge.find_map(nodes.len(), |i| {
                        let child_mount = child_mounts[i];
                        nodes[i].find_element_in_roots(child_mount, dom, target_id, edge)
                    })
                })
                .flatten(),
            Component(_) => {
                // Placement scans read the committed mount view, which is stable while a sibling
                // component is mid-diff; the live edge walk reads the scope's current render output.
                if committed_component_view {
                    let root_mount = dom.mounted_dynamic_component_root_mount(mount, idx)?;
                    let view = dom.current_mounted_view(root_mount)?;
                    view.find_element_in_roots(root_mount, dom, target_id, edge)
                } else {
                    let scope_id = dom.unchecked_mounted_dynamic_component_scope(mount, idx);
                    let root = live_component_root(dom, scope_id)?;
                    root.find_element_in_roots(root.mount(), dom, target_id, edge)
                }
            }
        }
    }

    fn dynamic_component_targets_other_render_target(
        &self,
        mount: MountId,
        idx: usize,
        dom: &VirtualDom,
    ) -> bool {
        if !matches!(&self.dynamic_node_values()[idx], Component(_)) {
            return false;
        }

        let scope_id = dom.unchecked_mounted_dynamic_component_scope(mount, idx);
        dom.runtime.get_state(scope_id).target_id() != dom.current_render_target_id()
    }

    /// Create this vnode under explicit render/logical parents.
    ///
    /// Invariant: when `to` is `Some`, the new mount is foreground-renderable and every static
    /// template root receives a mounted root id.
    pub(crate) fn create_mounted(
        &self,
        dom: &mut VirtualDom,
        render_parent: Option<MountId>,
        logical_parent: Option<MountId>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> CreatedVNode {
        let mut state = DiffState::new(dom, to);
        let target_id = state.dom.current_render_target_id();
        let mount = state
            .dom
            .create_mount(self, render_parent, logical_parent, target_id);
        self.finish_create(mount, &mut state)
    }

    /// Re-emit an already-mounted (background) subtree to the foreground writer,
    /// reusing its existing mount and child scopes rather than allocating fresh
    /// ones. The caller guarantees `mount` currently holds this same-template
    /// vnode.
    pub(crate) fn recreate_with_mount(
        &self,
        dom: &mut VirtualDom,
        mount: MountId,
        render_parent: Option<MountId>,
        logical_parent: Option<MountId>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> CreatedVNode {
        let mut state = DiffState::new(dom, to);
        let target_id = state.dom.current_render_target_id();
        state
            .dom
            .reuse_mount(mount, render_parent, logical_parent, target_id);
        self.finish_create(mount, &mut state)
    }

    /// Materialize this node for a slot that may already hold a background-rendered
    /// mount. When `old_mount` holds the same template, reuse it in place (keeping
    /// its scope subtree) via [`Self::recreate_with_mount`]; otherwise allocate a
    /// fresh mount and remove the old one.
    pub(crate) fn create_or_reuse_mount(
        &self,
        dom: &mut VirtualDom,
        old_mount: Option<MountId>,
        render_parent: Option<MountId>,
        logical_parent: Option<MountId>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> CreatedVNode {
        let reuse = old_mount.filter(|&mount| {
            dom.current_mounted_view(mount)
                .is_some_and(|old| old.template() == self.template())
        });
        match reuse {
            Some(mount) => self.recreate_with_mount(dom, mount, render_parent, logical_parent, to),
            None => {
                let created = self.create_mounted(dom, render_parent, logical_parent, to);
                if let Some(old_mount) = old_mount {
                    dom.current_mounted_view(old_mount)
                        .expect("mount")
                        .remove_node(old_mount, dom, None);
                }
                created
            }
        }
    }

    fn finish_create(&self, mount: MountId, state: &mut DiffState<'_, '_, '_, '_>) -> CreatedVNode {
        debug_assert!(
            state.to.is_none() || state.dom.mount_should_render(mount),
            "background mounts must be created without renderer writes"
        );

        let nodes_created = self.create_root_children(mount, state);
        self.fill_nested_dynamic_slots(mount, state);

        state.dom.commit_mount(mount, self);
        CreatedVNode {
            nodes: nodes_created,
            mount,
        }
    }
}

impl VNode {
    fn create_root_children(&self, mount: MountId, state: &mut DiffState<'_, '_, '_, '_>) -> usize {
        let mut nodes_created = 0;

        for child in self.children() {
            match child {
                VNodeChild::Dynamic(anchor) => {
                    nodes_created += self.create_dynamic_anchor_nodes(mount, anchor, state);
                }
                VNodeChild::Element(element) => {
                    if let Some(to) = state.to.as_deref_mut() {
                        let root_anchor_idx = element.anchor_index().expect("root element");
                        let id =
                            self.load_template_root(root_anchor_idx, element.op(), state.dom, to);
                        self.assign_template_anchor_ids(mount, root_anchor_idx, id, state.dom, to);
                        nodes_created += 1;
                    }
                }
                VNodeChild::Text(text) => {
                    if let Some(to) = state.to.as_deref_mut() {
                        let root_anchor_idx = text.anchor_index().expect("root text");
                        let id = self.load_template_root(root_anchor_idx, text.op(), state.dom, to);
                        self.assign_template_anchor_ids(mount, root_anchor_idx, id, state.dom, to);
                        nodes_created += 1;
                    }
                }
            }
        }

        nodes_created
    }

    fn fill_nested_dynamic_slots(&self, mount: MountId, state: &mut DiffState<'_, '_, '_, '_>) {
        for anchor in self.dynamic_anchors() {
            if anchor.attrs().len() > 0
                && let Some(to) = state.to.as_deref_mut()
            {
                self.write_attr_anchor(mount, anchor, state.dom, to);
            }

            if anchor.nodes().len() > 0 && !anchor.is_root_level() {
                self.load_dynamic_anchor(mount, anchor, state);
            }
        }
    }

    /// Create one dynamic node value in an already allocated mount slot.
    ///
    /// Invariant: `idx` is allocated in `mount` and matches a dynamic node in `self`.
    pub(crate) fn create_dynamic_node(
        &self,
        mount: MountId,
        idx: usize,
        state: &mut DiffState<'_, '_, '_, '_>,
    ) -> usize {
        use DynamicNode::*;
        let parent = Some(mount);
        let node = &self.dynamic_node_values()[idx];
        match node {
            Component(c) => self.create_component_node(mount, idx, c, state),
            Fragment(frag) => {
                if let Some(mounts) =
                    state
                        .dom
                        .try_with_mounted_fragment_children(mount, idx, frag.len(), |mounts| {
                            mounts.to_vec()
                        })
                {
                    let mut nodes = 0;
                    let children =
                        state
                            .dom
                            .begin_mounted_fragment_children(mount, idx, frag.len());
                    for (idx, (child, child_mount)) in frag.iter().zip(mounts).enumerate() {
                        let created = child.create_or_reuse_mount(
                            state.dom,
                            Some(child_mount),
                            parent,
                            parent,
                            state.to.as_deref_mut(),
                        );
                        state
                            .dom
                            .set_mounted_fragment_child(children, idx, created.mount);
                        nodes += created.nodes;
                    }
                    state.dom.commit_mounted_fragment_children(children);
                    return nodes;
                }

                let children = state
                    .dom
                    .begin_mounted_fragment_children(mount, idx, frag.len());
                let nodes = state.dom.create_children_with_mounts(
                    state.to.as_deref_mut(),
                    frag,
                    parent,
                    parent,
                    |dom, idx, child_mount| {
                        dom.set_mounted_fragment_child(children, idx, child_mount)
                    },
                );
                state.dom.commit_mounted_fragment_children(children);
                nodes
            }
            Text(text) => self.create_dynamic_text_node(mount, idx, text, state),
        }
    }

    fn create_dynamic_text_node(
        &self,
        mount: MountId,
        idx: usize,
        text: &VText,
        state: &mut DiffState<'_, '_, '_, '_>,
    ) -> usize {
        // If we are diffing suspended nodes and are not outputting mutations, we can skip it.
        let Some(to) = state.to.as_deref_mut() else {
            return 0;
        };

        let target_id = state.dom.current_render_target_id();
        let id = state.dom.next_element_in_target(target_id);
        state.dom.set_mounted_dynamic_text_node(mount, idx, id);
        to.create_text(&text.value);
        to.set_id(id.element_id());
        1
    }

    fn create_dynamic_anchor_nodes(
        &self,
        mount: MountId,
        anchor: DynamicAnchor<'_>,
        state: &mut DiffState<'_, '_, '_, '_>,
    ) -> usize {
        anchor
            .nodes()
            .map(|slot| self.create_dynamic_node(mount, slot.index(), state))
            .sum()
    }

    fn load_dynamic_anchor(
        &self,
        mount: MountId,
        anchor: DynamicAnchor<'_>,
        state: &mut DiffState<'_, '_, '_, '_>,
    ) {
        if !state.has_writer() {
            for slot in anchor.nodes() {
                self.create_dynamic_node(mount, slot.index(), state);
            }
            return;
        }

        let first_slot = anchor.nodes().next().expect("dynamic anchor has nodes");
        let context = state.context();
        let site = insertion_site_for_slot(mount, first_slot, state.dom, context);
        let runtime = state.dom.runtime.clone();
        let dom = &mut *state.dom;
        let to = state.to.as_deref_mut().expect("writer checked");
        at_site(site, to, runtime, |to| {
            let mut state = DiffState::new_with_context(dom, Some(to), context);
            self.create_dynamic_anchor_nodes(mount, anchor, &mut state)
        });
    }

    fn write_attr_anchor(
        &self,
        mount: MountId,
        anchor: DynamicAnchor<'_>,
        dom: &mut VirtualDom,
        to: &mut dyn WriteMutations,
    ) {
        let id = dom.unchecked_mounted_anchor_node(mount, anchor.anchor_index());
        with_id(to, id.element_id(), |to| {
            for attr in anchor.attrs().flat_map(|slot| slot.attrs()) {
                Self::write_attribute_to_current(attr, id, mount, dom, to);
            }
        });
    }

    fn load_template_root(
        &self,
        root_anchor_idx: usize,
        root_op: usize,
        dom: &mut VirtualDom,
        to: &mut dyn WriteMutations,
    ) -> MountedElementId {
        let target_id = dom.current_render_target_id();
        let id = dom.next_element_in_target(target_id);

        let static_root = self
            .template()
            .static_node(root_op)
            .expect("bad static root");
        if to.can_cache_template_roots() {
            let template_id =
                match dom.cached_template_root(target_id, *self.template(), root_anchor_idx) {
                    Some(id) => id,
                    None => {
                        let id = dom.allocate_template_root(
                            target_id,
                            *self.template(),
                            root_anchor_idx,
                        );
                        create_static_prototype(static_root, to);
                        to.set_id(id.element_id());
                        to.pop();
                        id
                    }
                };
            to.push_id(template_id.element_id());
            WriteMutations::clone(to);
        } else {
            create_static_prototype(static_root, to);
        }
        to.set_id(id.element_id());
        id
    }

    fn assign_template_anchor_ids(
        &self,
        mount: MountId,
        root_anchor_idx: usize,
        root_id: MountedElementId,
        dom: &mut VirtualDom,
        to: &mut dyn WriteMutations,
    ) {
        let mut assigned_paths = Vec::new();

        for target in self.static_anchor_targets_under(root_anchor_idx) {
            let anchor_idx = target.anchor_index;
            let path = target.path;

            if let Some((_, id)) = assigned_paths
                .iter()
                .find(|(assigned_path, _)| *assigned_path == path)
            {
                dom.set_mounted_anchor_node(mount, anchor_idx, *id);
                continue;
            }

            let id = if path.is_root() {
                root_id
            } else {
                let target_id = dom.current_render_target_id();
                let id = dom.next_element_in_target(target_id);
                with_consumed_id(to, root_id.element_id(), |to| {
                    for segment in path.segments().skip(1) {
                        to.child(segment);
                    }
                    to.set_id(id.element_id());
                    to.pop();
                });
                id
            };

            assigned_paths.push((path, id));
            dom.set_mounted_anchor_node(mount, anchor_idx, id);
        }
    }
}

fn anchor_static_target(anchor: &TemplateAnchor) -> Option<TemplatePath> {
    let path = anchor.static_path();
    (!path.is_empty()).then_some(path)
}

fn create_static_prototype(node: StaticTemplateNode<'_>, to: &mut dyn WriteMutations) -> usize {
    match node {
        StaticTemplateNode::Element(element) => {
            to.create_element(element.tag(), element.namespace());

            for attr in element.attributes() {
                let value = crate::AttributeValue::Text(attr.value.to_string());
                to.set_attribute(attr.name, attr.namespace, &value);
            }

            let mut children = 0;
            for child in element.children() {
                children += create_static_prototype(child, to);
            }

            if children > 0 {
                to.append_children(children);
            }
            1
        }
        StaticTemplateNode::Text(text) => {
            to.create_text(text.text());
            1
        }
    }
}

/// Look up the rendered root VNode for a component scope, for walking with
/// `find_element_in_roots` during placement. A mounted component slot may have no
/// live root after portal or hidden-branch cleanup; such slots cannot anchor placement.
fn live_component_root(dom: &VirtualDom, scope_id: ScopeId) -> Option<MountedVNode<'_>> {
    dom.get_scope(scope_id)?.try_mounted_root_node()
}
