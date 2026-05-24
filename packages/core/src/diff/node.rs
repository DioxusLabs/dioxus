use crate::{
    DynamicNode::*,
    TemplateNode, VNode, VirtualDom, WriteMutations,
    arena::ElementId,
    diff::{
        anchor::{
            Anchor, ElementEdge, anchor_before, anchor_for_slot, at_anchor, create_at_anchor,
        },
        context::{DiffFrame, DiffState},
    },
    fiber::Fiber,
    innerlude::{ElementPath, ElementRef, MountId, ScopeOrder},
    nodes::DynamicNode,
    scopes::ScopeId,
};
use core::iter::Peekable;

impl VNode {
    pub(super) fn reference_to_dynamic_node(&self, mount: MountId, idx: usize) -> ElementRef {
        let path = self.template.node_paths()[idx];
        ElementRef {
            path: ElementPath { path },
            mount,
        }
    }

    pub(crate) fn create_dynamic_node(
        &self,
        node: &DynamicNode,
        mount: MountId,
        idx: usize,
        state: &mut DiffState<'_, impl WriteMutations>,
    ) -> usize {
        use DynamicNode::*;
        let parent = Some(self.reference_to_dynamic_node(mount, idx));
        match node {
            Component(c) => self.create_component_node(mount, idx, c, parent, state),
            Fragment(frag) => state
                .dom
                .create_children(state.to.as_deref_mut(), frag, parent),
            Text(text) => {
                // If we are diffing suspended nodes and are not outputting mutations, we can skip it
                if let Some(to) = state.to.as_deref_mut() {
                    self.create_dynamic_text(mount, idx, text, state.dom, to)
                } else {
                    0
                }
            }
        }
    }

    fn create_dynamic_text(
        &self,
        mount: MountId,
        idx: usize,
        text: &crate::innerlude::VText,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        let id = dom.next_element_for_mount(mount);
        dom.set_mounted_dyn_node(mount, idx, id.0);
        to.create_text_node(&text.value, id);
        1
    }

    /// Mount all dynamic nodes that are descendants of this root template element.
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # let some_text = "hello world";
    /// # let some_value = "123";
    /// rsx! {
    ///     div { // We just wrote this node
    ///         // This is a dynamic slot
    ///         {some_value}
    ///
    ///         // Load this too
    ///         "{some_text}"
    ///     }
    /// };
    /// ```
    ///
    /// IMPORTANT: This function assumes that root node is the top node on the stack
    pub(super) fn load_dynamic_slots(
        &self,
        mount: MountId,
        dynamic_nodes_iter: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        root_idx: u8,
        state: &mut DiffState<'_, impl WriteMutations>,
    ) {
        let Some((start, [first, ..])) = dynamic_nodes_iter.peek().copied() else {
            return;
        };
        if *first != root_idx {
            return;
        }
        let mut end = start;
        // Every dynamic surfaced here lives under an Element/Text root (the
        // Dynamic-at-root case is handled by the sibling arm in
        // `create_with_parents`), so the path always has the root index plus
        // at least one child segment — `idx` advances `end` unconditionally.
        while let Some((idx, _path)) =
            dynamic_nodes_iter.next_if(|(_, path)| matches!(path, [idx, ..] if *idx == root_idx))
        {
            debug_assert!(_path.len() > 1, "nested dynamics under an Element root have path.len() > 1");
            end = idx;
        }

        // Reverse order keeps path-based insertions from invalidating the paths
        // of slots that have not been processed yet.
        for dynamic_node_id in (start..=end).rev() {
            let m = self.create_dynamic_node(
                &self.dynamic_nodes[dynamic_node_id],
                mount,
                dynamic_node_id,
                state,
            );
            if m > 0
                && let Some(to) = state.to.as_deref_mut()
            {
                let path = &self.template.node_paths()[dynamic_node_id][1..];
                to.insert_children_at_path(path, m);
            }
        }
    }

    /// After we have written a root element, we need to write all the attributes that are on the root node
    ///
    /// ```rust, ignore
    /// rsx! {
    ///     div { // We just wrote this node
    ///         class: "{class}", // We need to set these attributes
    ///         id: "{id}",
    ///         style: "{style}",
    ///     }
    /// }
    /// ```
    ///
    /// IMPORTANT: This function assumes that root node is the top node on the stack
    pub(super) fn write_attrs(
        &self,
        mount: MountId,
        dynamic_attributes_iter: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        root_idx: u8,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let mut last_path = None;
        let from_root_node = |(_, path): &(usize, &[u8])| path.first() == Some(&root_idx);
        while let Some((attribute_idx, attribute_path)) =
            dynamic_attributes_iter.next_if(from_root_node)
        {
            let attribute = &self.dynamic_attrs[attribute_idx];

            let id = match last_path {
                Some((path, id)) if path == attribute_path => id,
                _ => {
                    let id = self.assign_static_node_as_dynamic(mount, attribute_path, dom, to);
                    last_path = Some((attribute_path, id));
                    id
                }
            };

            for attr in &**attribute {
                self.write_attribute(attribute_path, attr, id, mount, dom, to);
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
        path: &'static [u8],
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> ElementId {
        // This is just the root node. We already know it's id
        if let [root_idx] = path {
            return dom.get_mounted_root_node(mount, *root_idx as usize);
        }

        // The node is deeper in the template and we should create a new id for it
        let id = dom.next_element_for_mount(mount);

        to.assign_node_id(&path[1..], id);

        id
    }

    fn load_template_root(
        &self,
        mount: MountId,
        root_idx: usize,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> ElementId {
        let id = dom.next_element_for_mount(mount);
        dom.set_mounted_root_node(mount, root_idx, id);
        to.load_template(self.template, root_idx, id);
        id
    }
}

impl VNode {
    pub(crate) fn diff_node(
        &self,
        new: &VNode,
        dom: &mut VirtualDom,
        to: Option<&mut impl WriteMutations>,
    ) {
        let mut state = DiffState::new(dom, to);
        DiffFrame::new(self.mount.get(), self, new).diff_into(&mut state);
    }
}

impl<'a> DiffFrame<'a> {
    pub(crate) fn diff_into<M: WriteMutations>(self, state: &mut DiffState<'_, M>) {
        let old = self.old;
        let new = self.new;

        let current_mount = self.mount;
        let writes_enabled = state.dom.fiber_should_render(current_mount)
            && state
                .dom
                .render_target_should_write(state.dom.mount_target_id(current_mount));
        let mut state = state.reborrow_with_writes(writes_enabled);

        // If the templates are different, we need to replace the entire template
        if old.template != new.template {
            let parent = state.dom.get_mounted_parent(current_mount);
            return old.replace_inner(std::slice::from_ref(new), parent, &mut state, true);
        }

        let prev_mount = state.dom.claim_fiber_mount(old, new);
        state.enter_context(prev_mount, old, new);

        // If the templates are the same, we don't need to do anything, except copy over the mount information
        if old == new && !old.has_dirty_component_descendant(prev_mount, state.dom) {
            state.dom.commit_fiber_work(prev_mount, new);
            return;
        }

        // If the templates are the same, we can diff the attributes and children
        // Start with the attributes
        // Since the attributes are only side effects, we can skip diffing them entirely if the node is suspended and we aren't outputting mutations
        if let Some(to) = state.to.as_deref_mut() {
            old.diff_attributes(new, state.dom, to);
        }

        let mount_id = new.mount.get();
        for (dyn_node_idx, (old_dynamic, new_dynamic)) in old
            .dynamic_nodes
            .iter()
            .zip(new.dynamic_nodes.iter())
            .enumerate()
        {
            old.diff_dynamic_node(mount_id, dyn_node_idx, old_dynamic, new_dynamic, &mut state)
        }
        state.dom.commit_fiber_work(mount_id, new);
    }
}

impl VNode {
    fn has_dirty_component_descendant(&self, mount: MountId, dom: &VirtualDom) -> bool {
        self.dynamic_nodes
            .iter()
            .enumerate()
            .any(|(idx, node)| match node {
                Component(_) => {
                    let scope_id = ScopeId(dom.get_mounted_dyn_node(mount, idx));
                    dom.runtime.try_get_state(scope_id).is_some_and(|scope| {
                        dom.dirty_fibers
                            .contains(&ScopeOrder::new(scope.height(), scope_id))
                    })
                }
                Fragment(nodes) => nodes.iter().any(|node| {
                    let mount = node.mount.get();
                    mount.mounted() && node.has_dirty_component_descendant(mount, dom)
                }),
                Text(_) => false,
            })
    }

    fn diff_dynamic_node(
        &self,
        mount: MountId,
        idx: usize,
        old_node: &DynamicNode,
        new_node: &DynamicNode,
        state: &mut DiffState<'_, impl WriteMutations>,
    ) {
        tracing::trace!("diffing dynamic node from {old_node:?} to {new_node:?}");
        match (old_node, new_node) {
            (Text(old), Text(new)) => {
                // Diffing text is just a side effect, if we are diffing suspended nodes and are not outputting mutations, we can skip it
                if let Some(to) = state.to.as_deref_mut()
                    && old.value != new.value
                {
                    to.set_node_text(
                        &new.value,
                        ElementId(state.dom.get_mounted_dyn_node(mount, idx)),
                    );
                }
            }
            (Fragment(old), Fragment(new)) => self.diff_fragment(mount, idx, old, new, state),
            (Component(old), Component(new)) => {
                let scope_id = ScopeId(state.dom.get_mounted_dyn_node(mount, idx));
                self.diff_vcomponent(
                    mount,
                    idx,
                    new,
                    old,
                    scope_id,
                    Some(self.reference_to_dynamic_node(mount, idx)),
                    state,
                )
            }
            (old, new) => self.replace_dynamic_node_at_slot(mount, idx, old, new, state),
        };
    }

    fn replace_dynamic_node_at_slot<M: WriteMutations>(
        &self,
        mount: MountId,
        idx: usize,
        old: &DynamicNode,
        new: &DynamicNode,
        state: &mut DiffState<'_, M>,
    ) {
        let old_mount_value = state.dom.get_mounted_dyn_node(mount, idx);
        let old_has_live_dom = self.dynamic_node_has_live_dom(mount, idx, old, state.dom);
        if !old_has_live_dom {
            // Pass `None::<&mut M>` (the caller's writer type) instead of
            // `NoOpMutations` so this call reuses the caller's monomorphization.
            // A `NoOpMutations` mono here would carry copies of every
            // generic-driven function it transitively calls — `reclaim_roots`,
            // `remove_node_inner`, etc. — whose "writes enabled" arms are
            // unreachable in the NoOp mono, and that tanks per-monomorphization
            // region coverage.
            self.remove_dynamic_node(mount, state.dom, None::<&mut M>, true, idx, old);
        }

        let live_first = if old_has_live_dom {
            self.dynamic_node_first_element(mount, idx, old, state.dom)
        } else {
            None
        };
        let anchor = match live_first {
            Some(first) => Anchor::Before(first),
            None => anchor_for_slot(
                mount,
                self.template.node_paths()[idx],
                &[],
                state.dom,
                state.context(),
            ),
        };
        state.dom.set_mounted_dyn_node(mount, idx, usize::MAX);
        {
            let dom = &mut *state.dom;
            let to = state.to.as_deref_mut();
            at_anchor(anchor, to, |to| {
                let mut state = DiffState::new(dom, to);
                self.create_dynamic_node(new, mount, idx, &mut state)
            });
        }

        let new_mount_value = state.dom.get_mounted_dyn_node(mount, idx);
        if old_has_live_dom {
            state.dom.set_mounted_dyn_node(mount, idx, old_mount_value);
            self.remove_dynamic_node(mount, state.dom, state.to.as_deref_mut(), true, idx, old);
        }
        state.dom.set_mounted_dyn_node(mount, idx, new_mount_value);
    }

    /// Diff two fragments at a dynamic slot. Handles empty <-> non-empty transitions
    /// without using placeholders to anchor the slot position.
    fn diff_fragment(
        &self,
        mount: MountId,
        idx: usize,
        old: &[VNode],
        new: &[VNode],
        state: &mut DiffState<'_, impl WriteMutations>,
    ) {
        let parent = Some(self.reference_to_dynamic_node(mount, idx));
        match (old.is_empty(), new.is_empty()) {
            (true, true) => {}
            (true, false) => {
                // Empty → non-empty: stage new content at the slot's anchor.
                let own_mounts: Vec<MountId> = new.iter().map(|v| v.mount.get()).collect();
                let anchor = anchor_for_slot(
                    mount,
                    self.template.node_paths()[idx],
                    &own_mounts,
                    state.dom,
                    state.context(),
                );
                create_at_anchor(new, parent, anchor, state.dom, state.to.as_deref_mut());
            }
            (false, true) => {
                state.dom.remove_nodes(state.to.as_deref_mut(), old);
            }
            (false, false) => {
                state.diff_non_empty_fragment(old, new, parent);
            }
        }
    }

    /// Try to get the dynamic node and its index for a root node
    pub(crate) fn get_dynamic_root_node_and_id(
        &self,
        root_idx: usize,
    ) -> Option<(usize, &DynamicNode)> {
        let id = self.template.roots()[root_idx].dynamic_id()?;
        Some((id, &self.dynamic_nodes[id]))
    }

    pub(crate) fn find_first_element(&self, dom: &VirtualDom) -> Option<ElementId> {
        self.find_element_in_roots(
            dom,
            dom.current_render_target_id(),
            0..self.template.roots().len(),
            ElementEdge::First,
        )
    }

    pub(super) fn find_element_at_root_via_mount(
        &self,
        root_idx: usize,
        mount: MountId,
        dom: &VirtualDom,
        edge: ElementEdge,
    ) -> Option<ElementId> {
        self.find_element_at_root_in_target(
            root_idx,
            mount,
            dom.current_render_target_id(),
            dom,
            edge,
        )
    }

    fn find_element_at_root_in_target(
        &self,
        root_idx: usize,
        mount: MountId,
        target_id: crate::RenderTargetId,
        dom: &VirtualDom,
        edge: ElementEdge,
    ) -> Option<ElementId> {
        match self.get_dynamic_root_node_and_id(root_idx) {
            None if dom.mount_target_id(mount) == target_id => {
                // `self` may be a view whose template has more roots than the
                // fiber `mount` was actually created with (clones outlive
                // template changes); bail before indexing past
                // `MountedFiberState::root_ids`.
                if root_idx >= dom.mounted_root_count(mount) {
                    return None;
                }
                live_element_id(dom.get_mounted_root_node(mount, root_idx).0)
                    .filter(|id| dom.element_exists_in_target(target_id, *id))
            }
            None => None,
            Some((idx, Text(_))) if dom.mount_target_id(mount) == target_id => {
                // Same template-shape guard as the static-root branch above:
                // `self`'s template may declare more dynamic slots than the
                // fiber actually has.
                if idx >= dom.mounted_dyn_node_count(mount) {
                    return None;
                }
                live_element_id(dom.get_mounted_dyn_node(mount, idx))
                    .filter(|id| dom.element_exists_in_target(target_id, *id))
            }
            Some((_, Text(_))) => None,
            Some((_, Fragment(children))) => find_fragment_edge(children, dom, target_id, edge),
            Some((id, Component(_))) => {
                if id >= dom.mounted_dyn_node_count(mount) {
                    return None;
                }
                let scope_id = ScopeId(dom.get_mounted_dyn_node(mount, id));
                find_node_edge(
                    live_component_root(dom, scope_id)?,
                    dom,
                    target_id,
                    edge,
                )
            }
        }
    }

    pub(crate) fn find_last_element(&self, dom: &VirtualDom) -> Option<ElementId> {
        self.find_element_in_roots(
            dom,
            dom.current_render_target_id(),
            (0..self.template.roots().len()).rev(),
            ElementEdge::Last,
        )
    }

    fn has_live_dom(&self, dom: &VirtualDom) -> bool {
        let mount = self.mount.get();
        (0..self.template.roots().len())
            .any(|root_idx| self.root_has_live_dom(root_idx, mount, dom))
    }

    fn root_has_live_dom(&self, root_idx: usize, mount: MountId, dom: &VirtualDom) -> bool {
        // `mounted_root_count` / `mounted_dyn_node_count` are 0 when the
        // fiber state is gone (stale clone), which keeps the underlying
        // `get_mounted_*` calls in-bounds.
        match self.get_dynamic_root_node_and_id(root_idx) {
            None => {
                root_idx < dom.mounted_root_count(mount)
                    && live_element_id(dom.get_mounted_root_node(mount, root_idx).0)
                        .is_some_and(|id| dom.element_exists_for_mount(mount, id))
            }
            Some((idx, Text(_))) => {
                idx < dom.mounted_dyn_node_count(mount)
                    && live_element_id(dom.get_mounted_dyn_node(mount, idx))
                        .is_some_and(|id| dom.element_exists_for_mount(mount, id))
            }
            Some((_, Fragment(children))) => children.iter().any(|node| node.has_live_dom(dom)),
            Some((idx, Component(_))) => {
                idx < dom.mounted_dyn_node_count(mount) && {
                    let scope_id = ScopeId(dom.get_mounted_dyn_node(mount, idx));
                    dom.get_scope(scope_id)
                        .and_then(|scope| scope.try_root_node())
                        .is_some_and(|node| node.has_live_dom(dom))
                }
            }
        }
    }

    fn find_element_in_roots(
        &self,
        dom: &VirtualDom,
        target_id: crate::RenderTargetId,
        mut roots: impl Iterator<Item = usize>,
        edge: ElementEdge,
    ) -> Option<ElementId> {
        let mount = self.mount.get();
        // `find_element_at_root_in_target` is internally bounds-checked
        // against `mounted_root_count` / `mounted_dyn_node_count`, so it
        // safely returns `None` if this vnode's mount slot is gone.
        roots.find_map(|root_idx| {
            self.find_element_at_root_in_target(root_idx, mount, target_id, dom, edge)
        })
    }

    pub(crate) fn replace(
        &self,
        right: &[VNode],
        parent: Option<ElementRef>,
        dom: &mut VirtualDom,
        to: Option<&mut impl WriteMutations>,
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
        parent: Option<ElementRef>,
        dom: &mut VirtualDom,
        to: Option<&mut impl WriteMutations>,
    ) {
        let mut state = DiffState::new(dom, to);
        self.replace_inner(right, parent, &mut state, false)
    }

    pub(crate) fn replace_inner<M: WriteMutations>(
        &self,
        right: &[VNode],
        parent: Option<ElementRef>,
        state: &mut DiffState<'_, M>,
        destroy_component_state: bool,
    ) {
        let own_mounts: Vec<MountId> = right.iter().map(|v| v.mount.get()).collect();
        // When the old subtree has no live DOM and the boundary is hidden, we
        // skip emitting renderer mutations for both the create and remove
        // sides. We still call `create_at_anchor` so the new subtree gets its
        // mount slots populated — otherwise the caller (e.g. suspense's
        // background diff) may later read a mount that was never set.
        let suppress_mutations = self.should_suppress_mutations(state.dom, destroy_component_state);
        let anchor = anchor_before(self, &own_mounts, state.dom, state.context());
        let mut to_for_create = state.to.as_deref_mut();
        if suppress_mutations {
            to_for_create = None;
        }
        create_at_anchor(right, parent, anchor, state.dom, to_for_create);
        let to_for_remove = if suppress_mutations {
            None
        } else {
            state.to.as_deref_mut()
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
        current_scope_hidden_by_suspense(dom) && self.has_reclaimable_root(true)
    }

    fn has_reclaimable_root(&self, empty_text_only: bool) -> bool {
        self.template.roots().iter().any(|root| match root {
            TemplateNode::Dynamic { id } => match &self.dynamic_nodes[*id] {
                Component(_) => !empty_text_only,
                Text(text) => text.value.is_empty(),
                _ => false,
            },
            _ => !empty_text_only,
        })
    }

    /// Remove a node from the dom.
    pub(crate) fn remove_node<M: WriteMutations>(&self, dom: &mut VirtualDom, to: Option<&mut M>) {
        self.remove_node_inner(dom, to, true)
    }

    /// Remove a node, but only maybe destroy the component state of that node. During suspense, we need to remove a node from the real dom without wiping the component state
    pub(crate) fn remove_node_inner<M: WriteMutations>(
        &self,
        dom: &mut VirtualDom,
        to: Option<&mut M>,
        destroy_component_state: bool,
    ) {
        // Every caller (replace_inner, remove_nodes, Fragment removal,
        // scope cleanup) only reaches here with vnodes that went through
        // `create_with_parents` and have live mount slots in the fiber
        // registry. A PLACEHOLDER `mount` would mean a vnode was built but
        // never mounted, which can't happen mid-diff — `build_vnode` /
        // `claim_fiber_mount` always assign a live MountId before anything
        // tries to remove it.
        let mount = self.mount.get();
        debug_assert!(
            mount.mounted(),
            "remove_node_inner requires a live MountId"
        );

        // Clean up any attributes that have claimed a static node as dynamic for mount/unmounts
        // Will not generate mutations!
        self.reclaim_attributes(mount, dom);

        // Remove the nested dynamic nodes
        // We don't generate mutations for these, as they will be removed by the parent (in the next line)
        // But we still need to make sure to reclaim them from the arena and drop their hooks, etc
        self.remove_nested_dyn_nodes::<M>(mount, dom, destroy_component_state);

        // Clean up the roots, assuming we need to generate mutations for these
        // This is done last in order to preserve Node ID reclaim order (reclaim in reverse order of claim)
        self.reclaim_roots(mount, dom, to, destroy_component_state);

        if destroy_component_state {
            let mount = self.mount.take();
            let target_id = dom.mount_target_id(mount);
            dom.runtime.render_targets.borrow_mut()[target_id.0].remove_mounted_fiber(mount);
            // Remove the mount information
            dom.runtime.fibers.borrow_mut().remove(mount.0);
        }
    }

    fn reclaim_roots(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        mut to: Option<&mut impl WriteMutations>,
        destroy_component_state: bool,
    ) {
        for (idx, node) in self.template.roots().iter().enumerate() {
            if let Some(id) = node.dynamic_id() {
                let dynamic_node = &self.dynamic_nodes[id];
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
            } else {
                let id = dom.get_mounted_root_node(mount, idx);
                if id == ElementId::default() {
                    // Already reclaimed during a previous `move_node_to_background`.
                    continue;
                }
                if let Some(to) = to.as_deref_mut() {
                    to.remove_node(id);
                }
                dom.reclaim_for_mount(mount, id);
                dom.set_mounted_root_node(mount, idx, ElementId::default());
            }
        }
    }

    fn remove_nested_dyn_nodes<M: WriteMutations>(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        destroy_component_state: bool,
    ) {
        for (idx, dyn_node) in self.dynamic_nodes.iter().enumerate() {
            // Roots are cleaned up automatically above; non-root nested dynamic nodes get cleaned here.
            if self.template.node_paths()[idx].len() > 1 {
                self.remove_dynamic_node(
                    mount,
                    dom,
                    Option::<&mut M>::None,
                    destroy_component_state,
                    idx,
                    dyn_node,
                )
            }
        }
    }

    fn remove_dynamic_node(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        mut to: Option<&mut impl WriteMutations>,
        destroy_component_state: bool,
        idx: usize,
        node: &DynamicNode,
    ) {
        match node {
            Component(_comp) => {
                let scope_id = ScopeId(dom.get_mounted_dyn_node(mount, idx));
                dom.remove_component_node(to, destroy_component_state, scope_id);
            }
            Text(_) => {
                let Some(id) = live_element_id(dom.get_mounted_dyn_node(mount, idx)) else {
                    // No DOM was ever materialized for this text (e.g. it was rendered
                    // into a background-suspended subtree) or it was already reclaimed
                    // via a prior `move_node_to_background`. Skip emission/reclaim.
                    return;
                };
                if let Some(to) = to {
                    to.remove_node(id);
                }
                dom.reclaim_for_mount(mount, id);
                dom.set_mounted_dyn_node(mount, idx, usize::MAX);
            }
            Fragment(nodes) => {
                for node in nodes.iter() {
                    node.remove_node_inner(dom, to.as_deref_mut(), destroy_component_state);
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
                let scope_id = ScopeId(dom.get_mounted_dyn_node(mount, idx));
                dom.get_scope(scope_id)
                    .and_then(|scope| scope.try_root_node())
                    .is_some_and(|node| node.has_live_dom(dom))
            }
            Text(_) => live_element_id(dom.get_mounted_dyn_node(mount, idx))
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
                let scope_id = ScopeId(dom.get_mounted_dyn_node(mount, idx));
                let root = live_component_root(dom, scope_id)
                    .expect("dynamic_node_first_element runs only when has_live_dom asserted the component scope is live and rendered");
                find_node_edge(root, dom, target_id, ElementEdge::First)
            }
            Text(_) => {
                debug_assert_eq!(
                    dom.mount_target_id(mount),
                    target_id,
                    "Text dynamic node's mount target must match the current render target"
                );
                live_element_id(dom.get_mounted_dyn_node(mount, idx))
                    .filter(|id| dom.element_exists_in_target(target_id, *id))
            }
            Fragment(nodes) => find_fragment_edge(nodes, dom, target_id, ElementEdge::First),
        }
    }

    pub(super) fn reclaim_attributes(&self, mount: MountId, dom: &mut VirtualDom) {
        let mut reclaimed_id = None;
        for (idx, path) in self.template.attr_paths().iter().enumerate() {
            // We clean up the roots in the next step, so don't worry about them here
            if path.len() <= 1 {
                continue;
            }

            // only reclaim the new element if it's different from the previous one
            let id = dom.get_mounted_dyn_attr(mount, idx);
            if id != ElementId::default() && Some(id) != reclaimed_id {
                dom.reclaim_for_mount(mount, id);
                reclaimed_id = Some(id);
            }
            dom.set_mounted_dyn_attr(mount, idx, ElementId::default());
        }
    }

    /// Create this rsx block. This will create scopes from components that this rsx block contains, but it will not write anything to the DOM.
    pub(crate) fn create(
        &self,
        dom: &mut VirtualDom,
        parent: Option<ElementRef>,
        to: Option<&mut impl WriteMutations>,
    ) -> usize {
        self.create_with_parents(dom, parent, parent, to)
    }

    /// Create this rsx block with separate renderer and logical parents.
    pub(crate) fn create_with_parents(
        &self,
        dom: &mut VirtualDom,
        render_parent: Option<ElementRef>,
        logical_parent: Option<ElementRef>,
        to: Option<&mut impl WriteMutations>,
    ) -> usize {
        let mut state = DiffState::new(dom, to);
        // Get the most up to date template
        let template = self.template;

        // Initialize the mount information for this vnode if it isn't already mounted
        if !self.mount.get().mounted() {
            let target_id = render_parent
                .map(|parent| state.dom.mount_target_id(parent.mount))
                .unwrap_or_else(|| state.dom.current_render_target_id());
            let mut mounts = state.dom.runtime.fibers.borrow_mut();
            let entry = mounts.vacant_entry();
            let mount = MountId(entry.key());
            self.mount.set(mount);
            tracing::trace!(?self, ?mount, "creating template");
            entry.insert(Fiber::new(
                self.clone(),
                render_parent,
                logical_parent,
                target_id,
            ));
            drop(mounts);
            state.dom.runtime.render_targets.borrow_mut()[target_id.0].create_mounted_fiber(
                mount,
                template.roots().len(),
                template.attr_paths().len(),
                template.node_paths().len(),
            );
        }

        // Walk the roots, creating nodes and assigning IDs
        // nodes in an iterator of (dynamic_node_index, path) and attrs in an iterator of (attr_index, path)
        let mut nodes = template.node_paths().iter().copied().enumerate().peekable();
        let mut attrs = template.attr_paths().iter().copied().enumerate().peekable();

        // Get the mounted id of this block
        // At this point, we should have already mounted the block
        let mount = self.mount.get();
        if !state.dom.fiber_should_render(mount)
            || !state
                .dom
                .render_target_should_write(state.dom.mount_target_id(mount))
        {
            state.to = None;
        }
        debug_assert!(
            state.dom.runtime.fibers.borrow().contains(mount.0),
            "Tried to find mount {:?} in dom.fibers, but it wasn't there",
            mount
        );

        // Go through each root node and create the node, adding it to the stack.
        // Each node already exists in the template, so we can just clone it from the template

        // And return the number of nodes we created on the stack
        let nodes_created = template
            .roots()
            .iter()
            .enumerate()
            .map(|(root_idx, root)| match root {
                TemplateNode::Dynamic { id } => {
                    // Take a dynamic node off the depth first iterator
                    nodes.next().unwrap();
                    // Then mount the node
                    self.create_dynamic_node(&self.dynamic_nodes[*id], mount, *id, &mut state)
                }
                // For static text and element nodes, just load the template root. This may be a placeholder or just a static node. We now know that each root node has a unique id
                TemplateNode::Text { .. } | TemplateNode::Element { .. } => {
                    let writes_enabled = state.to.is_some();
                    if let Some(to) = state.to.as_deref_mut() {
                        self.load_template_root(mount, root_idx, state.dom, to);
                    }

                    // If this is an element, load in all of the placeholder or dynamic content under this root element too
                    if matches!(root, TemplateNode::Element { .. }) {
                        // !!VERY IMPORTANT!!
                        // Write out all attributes before we load the children. Loading the children will change paths we rely on
                        // to assign ids to elements with dynamic attributes
                        if let Some(to) = state.to.as_deref_mut() {
                            self.write_attrs(mount, &mut attrs, root_idx as u8, state.dom, to);
                        }
                        // This operation relies on the fact that the root node is the top node on the stack so we need to do it here
                        self.load_dynamic_slots(mount, &mut nodes, root_idx as u8, &mut state);
                    }

                    // This creates one node on the stack if writes are enabled.
                    usize::from(writes_enabled)
                }
            })
            .sum();
        // Now that all descendants have been mounted and their `Cell<MountId>`
        // slots populated, snapshot ourselves into the fiber. Using a
        // deep-clone here gives the snapshot its own per-vnode cells, so a
        // later `claim_fiber_mount` against a sibling subtree can't mutate
        // them out from under anchor lookups that read this fiber.
        state.dom.commit_fiber_work(mount, self);
        nodes_created
    }
}

fn current_scope_hidden_by_suspense(dom: &VirtualDom) -> bool {
    dom.runtime
        .try_current_scope_id()
        .and_then(|scope| dom.runtime.try_get_state(scope))
        .is_some_and(|scope| !scope.suspense_location().hidden_by().is_empty())
}

/// Look up the rendered root VNode for a component scope, returning `None`
/// when the scope has been dropped (mid-diff during a sibling removal or
/// suspense transition) or hasn't rendered yet. Callers walk the returned
/// root with `find_node_edge` for anchor placement.
fn live_component_root(dom: &VirtualDom, scope_id: ScopeId) -> Option<&VNode> {
    dom.get_scope(scope_id)?.try_root_node()
}

fn live_element_id(raw: usize) -> Option<ElementId> {
    (raw != 0 && raw != usize::MAX).then_some(ElementId(raw))
}

fn find_fragment_edge(
    children: &[VNode],
    dom: &VirtualDom,
    target_id: crate::RenderTargetId,
    edge: ElementEdge,
) -> Option<ElementId> {
    match edge {
        ElementEdge::First => children
            .iter()
            .find_map(|child| find_node_edge(child, dom, target_id, edge)),
        ElementEdge::Last => children
            .iter()
            .rev()
            .find_map(|child| find_node_edge(child, dom, target_id, edge)),
    }
}

fn find_node_edge(
    node: &VNode,
    dom: &VirtualDom,
    target_id: crate::RenderTargetId,
    edge: ElementEdge,
) -> Option<ElementId> {
    match edge {
        ElementEdge::First => node.find_element_in_roots(
            dom,
            target_id,
            0..node.template.roots().len(),
            ElementEdge::First,
        ),
        ElementEdge::Last => node.find_element_in_roots(
            dom,
            target_id,
            (0..node.template.roots().len()).rev(),
            ElementEdge::Last,
        ),
    }
}

