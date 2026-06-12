use crate::{
    ElementId, RenderTargetId, ScopeId, VNode,
    arena::{ElementRef, MountId, MountedNodeState},
    virtual_dom::VirtualDom,
};

/// Whether a mount is allowed to write renderer mutations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RenderMode {
    Foreground,
    Background,
}

/// Persistent render identity for one mounted `VNode`.
///
/// A mount owns the renderer ids and dynamic child bindings for an rsx block.
/// `node` is the committed view used after diffing for event dispatch, tree
/// inspection, and the next render pass.
#[derive(Debug)]
pub(crate) struct Mount {
    /// The physical parent used for renderer placement and anchors.
    render_parent: Option<ElementRef>,

    /// The logical parent used for context tree event bubbling.
    logical_parent: Option<ElementRef>,

    /// The render target this mount is materialized into.
    target_id: RenderTargetId,

    /// The committed view used for events and mounted tree inspection.
    node: VNode,

    /// Suspense can keep a primary branch alive while its fallback is visible.
    /// Background mounts may update their virtual tree, but they must not write
    /// renderer mutations until they are promoted back to the foreground.
    mode: RenderMode,
}

impl Mount {
    pub(crate) fn new(
        node: VNode,
        render_parent: Option<ElementRef>,
        logical_parent: Option<ElementRef>,
        target_id: RenderTargetId,
    ) -> Self {
        Self {
            render_parent,
            logical_parent,
            target_id,
            node,
            mode: RenderMode::Foreground,
        }
    }

    pub(crate) fn logical_parent(&self) -> Option<ElementRef> {
        self.logical_parent
    }

    pub(crate) fn node(&self) -> &VNode {
        &self.node
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MountedDynamicNodeSlot(usize);

impl MountedDynamicNodeSlot {
    const PLACEHOLDER: Self = Self(usize::MAX);
}

impl VirtualDom {
    pub(crate) fn create_mount(
        &mut self,
        node: &VNode,
        render_parent: Option<ElementRef>,
        logical_parent: Option<ElementRef>,
        root_count: usize,
        attr_count: usize,
        dynamic_count: usize,
    ) -> MountId {
        let target_id = render_parent
            .map(|parent| self.mount_target_id(parent.mount))
            .unwrap_or_else(|| self.current_render_target_id());

        let mut mounts = self.runtime.mounts.borrow_mut();
        let entry = mounts.vacant_entry();
        let mount = MountId(entry.key());
        node.mount.set(mount);
        entry.insert(Mount::new(
            node.clone(),
            render_parent,
            logical_parent,
            target_id,
        ));
        drop(mounts);

        self.runtime.render_targets.borrow_mut()[target_id.0].create_mounted_node(
            mount,
            root_count,
            attr_count,
            dynamic_count,
        );

        mount
    }

    pub(crate) fn remove_mount(&mut self, mount: MountId) {
        let target_id = self.mount_target_id(mount);
        self.runtime.render_targets.borrow_mut()[target_id.0].remove_mounted_node(mount);
        self.runtime.mounts.borrow_mut().remove(mount.0);
    }

    pub(crate) fn mount_target_id(&self, mount: MountId) -> RenderTargetId {
        // Every caller has a live `mount` — either freshly allocated via
        // `next_element_for_mount` / mount creation, or the result of
        // `claim_mount` on a previously-mounted vnode. A PLACEHOLDER
        // here would indicate a stray ref the diff never produces.
        self.runtime
            .mounts
            .borrow()
            .get(mount.0)
            .map(|mount| mount.target_id)
            .expect("mounted mount record should exist")
    }

    pub(crate) fn get_mounted_parent(&self, mount: MountId) -> Option<ElementRef> {
        self.mounted_render_parent(mount)
    }

    pub(crate) fn mounted_render_parent(&self, mount: MountId) -> Option<ElementRef> {
        self.runtime
            .mounts
            .borrow()
            .get(mount.0)
            .map(|mount| mount.render_parent)
            .expect("mounted mount record should exist")
    }

    pub(crate) fn get_mounted_logical_parent(&self, mount: MountId) -> Option<ElementRef> {
        self.runtime
            .mounts
            .borrow()
            .get(mount.0)
            .map(|mount| mount.logical_parent)
            .expect("mounted mount record should exist")
    }

    /// Number of template roots this `mount`'s mount was created with.
    /// Anchor lookups that walk a view's `template.roots()` may iterate
    /// beyond what the mount actually has — e.g. when the view was a clone
    /// whose template grew between renders — and the underlying
    /// `MountedNodeState::root_ids` would panic on out-of-range indexing.
    pub(crate) fn mounted_root_count(&self, mount: MountId) -> usize {
        self.mounted_node_state(mount, |state| state.root_ids.len())
            .expect("mounted mount state should exist")
    }

    /// Number of dynamic-node slots this `mount`'s mount was created with.
    /// Same guard rail as [`Self::mounted_root_count`], but for
    /// `MountedNodeState::mounted_dynamic_nodes`.
    pub(crate) fn mounted_dyn_node_count(&self, mount: MountId) -> usize {
        self.mounted_node_state(mount, |state| state.mounted_dynamic_nodes.len())
            .expect("mounted mount state should exist")
    }

    fn get_mounted_dynamic_node_slot_raw(&self, mount: MountId, dyn_node_idx: usize) -> usize {
        self.with_mounted_node_state(mount, |state| state.mounted_dynamic_nodes[dyn_node_idx])
    }

    fn set_mounted_dynamic_node_slot_raw(&self, mount: MountId, dyn_node_idx: usize, value: usize) {
        self.with_mounted_node_state_mut(mount, |state| {
            state.mounted_dynamic_nodes[dyn_node_idx] = value;
        });
    }

    pub(crate) fn get_mounted_dynamic_node_slot(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> MountedDynamicNodeSlot {
        MountedDynamicNodeSlot(self.get_mounted_dynamic_node_slot_raw(mount, dyn_node_idx))
    }

    pub(crate) fn set_mounted_dynamic_node_slot(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        value: MountedDynamicNodeSlot,
    ) {
        self.set_mounted_dynamic_node_slot_raw(mount, dyn_node_idx, value.0);
    }

    pub(crate) fn clear_mounted_dynamic_node_slot(&self, mount: MountId, dyn_node_idx: usize) {
        self.set_mounted_dynamic_node_slot(
            mount,
            dyn_node_idx,
            MountedDynamicNodeSlot::PLACEHOLDER,
        );
    }

    pub(crate) fn get_mounted_dynamic_text_node(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> ElementId {
        ElementId(self.get_mounted_dynamic_node_slot(mount, dyn_node_idx).0)
    }

    pub(crate) fn set_mounted_dynamic_text_node(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        value: ElementId,
    ) {
        self.set_mounted_dynamic_node_slot(mount, dyn_node_idx, MountedDynamicNodeSlot(value.0));
    }

    pub(crate) fn clear_mounted_dynamic_text_node(&self, mount: MountId, dyn_node_idx: usize) {
        self.set_mounted_dynamic_text_node(mount, dyn_node_idx, ElementId::PLACEHOLDER);
    }

    pub(crate) fn get_mounted_dynamic_component_scope(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> ScopeId {
        ScopeId(self.get_mounted_dynamic_node_slot(mount, dyn_node_idx).0)
    }

    pub(crate) fn set_mounted_dynamic_component_scope(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        value: ScopeId,
    ) {
        self.set_mounted_dynamic_node_slot(mount, dyn_node_idx, MountedDynamicNodeSlot(value.0));
    }

    pub(crate) fn clear_mounted_dynamic_component_scope(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) {
        self.set_mounted_dynamic_component_scope(mount, dyn_node_idx, ScopeId::PLACEHOLDER);
    }

    pub(crate) fn get_mounted_dyn_attr(&self, mount: MountId, dyn_attr_idx: usize) -> ElementId {
        self.with_mounted_node_state(mount, |state| state.mounted_attributes[dyn_attr_idx])
    }

    pub(crate) fn set_mounted_dyn_attr(
        &self,
        mount: MountId,
        dyn_attr_idx: usize,
        value: ElementId,
    ) {
        self.with_mounted_node_state_mut(mount, |state| {
            state.mounted_attributes[dyn_attr_idx] = value;
        });
    }

    pub(crate) fn get_mounted_root_node(&self, mount: MountId, root_idx: usize) -> ElementId {
        self.with_mounted_node_state(mount, |state| state.root_ids[root_idx])
    }

    pub(crate) fn set_mounted_root_node(&self, mount: MountId, root_idx: usize, value: ElementId) {
        self.with_mounted_node_state_mut(mount, |state| {
            state.root_ids[root_idx] = value;
        });
    }

    pub(crate) fn current_mounted_view(&self, mount: MountId) -> Option<VNode> {
        // Hand out a deep clone so anchor lookups that descend into the
        // returned tree can't observe descendant mount cells being mutated
        // by a sibling diff's `claim_mount`.
        self.runtime
            .mounts
            .borrow()
            .get(mount.0)
            .map(|mount| mount.node.deep_clone_preserving_mounts())
    }

    pub(crate) fn set_mount_mode(&self, mount: MountId, mode: RenderMode) {
        self.runtime.mounts.borrow_mut()[mount.0].mode = mode;
    }

    pub(crate) fn mount_should_render(&self, mount: MountId) -> bool {
        // For an unmounted `mount` (`mount.0 == usize::MAX`),
        // `mounts.get(mount.0)` returns `None` and the `is_none_or` predicate
        // short-circuits to `true` — same answer as an explicit early return,
        // so the explicit branch isn't needed.
        self.runtime
            .mounts
            .borrow()
            .get(mount.0)
            .is_none_or(|mount| mount.mode == RenderMode::Foreground)
    }

    pub(crate) fn claim_mount(&self, old: &VNode, new: &VNode) -> MountId {
        let mount = old.mount.take();
        new.mount.set(mount);
        mount
    }

    pub(crate) fn commit_mount(&self, mount: MountId, node: &VNode) {
        // Every caller commits work on a `mount` that's just been claimed via
        // `claim_mount` or freshly allocated in `create_with_parents` —
        // both produce live `MountId`s, never `PLACEHOLDER`. A `PLACEHOLDER`
        // here would index past the mount slab below and panic regardless.
        // Deep-clone so the committed snapshot owns its own per-vnode
        // `Cell<MountId>` slots. A subsequent diff that calls
        // `claim_mount` on descendant `old` vnodes would otherwise
        // mutate the shared `Rc<VNodeInner>` here too, and anchor lookups
        // that walk `mount.node` would see those descendants as unmounted.
        self.runtime.mounts.borrow_mut()[mount.0].node = node.deep_clone_preserving_mounts();
    }

    fn mounted_node_state<R>(
        &self,
        mount: MountId,
        with_state: impl FnOnce(&MountedNodeState) -> R,
    ) -> Option<R> {
        let target_id = self.mount_target_id(mount);
        let targets = self.runtime.render_targets.borrow();
        targets
            .get(target_id.0)
            .and_then(|target| target.mounts.get(mount.0))
            .and_then(|mount| mount.as_ref())
            .map(with_state)
    }

    fn with_mounted_node_state<R>(
        &self,
        mount: MountId,
        with_state: impl FnOnce(&MountedNodeState) -> R,
    ) -> R {
        self.mounted_node_state(mount, with_state)
            .expect("mounted mount state should exist")
    }

    fn with_mounted_node_state_mut<R>(
        &self,
        mount: MountId,
        with_state: impl FnOnce(&mut MountedNodeState) -> R,
    ) -> R {
        let target_id = self.mount_target_id(mount);
        let mut targets = self.runtime.render_targets.borrow_mut();
        let state = targets
            .get_mut(target_id.0)
            .and_then(|target| target.mounts.get_mut(mount.0))
            .and_then(|mount| mount.as_mut())
            .expect("mounted mount state should exist");
        with_state(state)
    }
}

/// A retained suspense branch.
///
/// Suspense keeps the hidden primary branch alive while the fallback branch is
/// visible. The root `VNode` is still the render output we diff, but the branch
/// also records the root mount identity so the boundary state is explicitly tied
/// to retained mount ownership instead of being just a parked vnode.
#[derive(Clone, Debug)]
pub(crate) struct SuspenseBranch {
    root: VNode,
    root_mount: MountId,
}

impl SuspenseBranch {
    pub(crate) fn new(root: VNode) -> Self {
        let root_mount = root.mount.get();
        // Deep-clone on the way in so the stored root has its own
        // `VNodeInner`. Subsequent diffs against this branch can take per-slot
        // mounts via `claim_mount` without modifying any `Cell<MountId>`
        // shared with the parent's props or `last_rendered_node`.
        let root = root.deep_clone_preserving_mounts();
        Self { root, root_mount }
    }

    pub(crate) fn root(&self) -> VNode {
        // And one more deep-clone on the way out, so each diff pass that
        // reads the branch gets a fresh tree to consume rather than mutating
        // the stored copy across renders.
        self.root.deep_clone_preserving_mounts()
    }

    pub(crate) fn root_mount(&self) -> MountId {
        self.root_mount
    }

    pub(crate) fn into_root(self) -> VNode {
        self.root
    }
}
