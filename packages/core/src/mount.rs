use crate::{
    RenderTargetId, ScopeId, VNode,
    arena::{MountId, MountRef, MountedDynamicNodeSlot, MountedElementId},
    virtual_dom::VirtualDom,
};

/// Whether a mount is allowed to write renderer mutations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RenderMode {
    Foreground,
    Background,
}

impl MountedDynamicNodeSlot {
    fn text(self) -> Option<MountedElementId> {
        match self {
            Self::Text(id) => Some(id),
            _ => None,
        }
    }

    fn component_scope(self) -> Option<ScopeId> {
        match self {
            Self::Component { scope, .. } => Some(scope),
            _ => None,
        }
    }

    fn component_root_mount(self) -> Option<MountId> {
        match self {
            Self::Component { root_mount, .. } => root_mount,
            _ => None,
        }
    }

    fn set_component_scope(&mut self, scope: ScopeId) {
        let root_mount = self.component_root_mount();
        *self = Self::Component { scope, root_mount };
    }

    fn set_component_root_mount(&mut self, root_mount: Option<MountId>) {
        let scope = self
            .component_scope()
            .expect("dynamic component scope slot should be mounted");
        *self = Self::Component { scope, root_mount };
    }
}

/// Persistent render identity for one mounted `VNode`.
///
/// A mount owns the renderer ids and dynamic child bindings for an rsx block.
/// `node` is the committed view used after diffing for event dispatch, tree
/// inspection, and the next render pass. A mount belongs to exactly one render
/// target, so its per-mount bindings live here rather than in a parallel
/// per-target table.
#[derive(Debug)]
pub(crate) struct Mount {
    /// The physical parent used for renderer placement.
    render_parent: Option<MountRef>,

    /// The logical parent used for context tree event bubbling.
    logical_parent: Option<MountRef>,

    /// The render target this mount is materialized into.
    target_id: RenderTargetId,

    /// The committed view used for events and mounted tree inspection.
    node: VNode,

    /// Suspense can keep a primary branch alive while its fallback is visible.
    /// Background mounts may update their virtual tree, but they must not write
    /// renderer mutations until they are promoted back to the foreground.
    mode: RenderMode,

    /// The renderer ids for the roots of this template, used when moving or
    /// removing roots from the renderer.
    root_ids: Box<[Option<MountedElementId>]>,

    /// The renderer element each dynamic attribute is mounted to.
    mounted_attributes: Box<[Option<MountedElementId>]>,

    /// The mounted target for each dynamic node slot.
    mounted_dynamic_nodes: Box<[MountedDynamicNodeSlot]>,
}

impl Mount {
    pub(crate) fn new(
        node: VNode,
        render_parent: Option<MountRef>,
        logical_parent: Option<MountRef>,
        target_id: RenderTargetId,
        root_count: usize,
        attr_count: usize,
        dynamic_count: usize,
    ) -> Self {
        Self {
            render_parent,
            logical_parent,
            target_id,
            node,
            mode: RenderMode::Foreground,
            root_ids: vec![None; root_count].into(),
            mounted_attributes: vec![None; attr_count].into(),
            mounted_dynamic_nodes: vec![MountedDynamicNodeSlot::Empty; dynamic_count].into(),
        }
    }

    pub(crate) fn dynamic_node_slot(&self, idx: usize) -> Option<MountedDynamicNodeSlot> {
        self.mounted_dynamic_nodes.get(idx).copied()
    }

    pub(crate) fn mounted_attribute(&self, idx: usize) -> Option<MountedElementId> {
        self.mounted_attributes.get(idx).copied().flatten()
    }

    pub(crate) fn logical_parent(&self) -> Option<MountRef> {
        self.logical_parent
    }

    pub(crate) fn node(&self) -> &VNode {
        &self.node
    }
}

impl VirtualDom {
    pub(crate) fn create_mount(
        &mut self,
        node: &VNode,
        render_parent: Option<MountRef>,
        logical_parent: Option<MountRef>,
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
        node.set_mounted_id(mount);
        entry.insert(Mount::new(
            node.clone(),
            render_parent,
            logical_parent,
            target_id,
            root_count,
            attr_count,
            dynamic_count,
        ));

        mount
    }

    pub(crate) fn remove_mount(&mut self, mount: MountId) {
        self.runtime.mounts.borrow_mut().remove(mount.0);
    }

    pub(crate) fn mount_target_id(&self, mount: MountId) -> RenderTargetId {
        // Every caller has a live `mount` — either freshly allocated via
        // `next_element_for_mount` / mount creation, or the result of
        // `claim_mount` on a previously-mounted vnode.
        self.with_mount(mount, |mount| mount.target_id)
    }

    pub(crate) fn get_mounted_parent(&self, mount: MountId) -> Option<MountRef> {
        self.mounted_render_parent(mount)
    }

    pub(crate) fn mounted_render_parent(&self, mount: MountId) -> Option<MountRef> {
        self.with_mount(mount, |mount| mount.render_parent)
    }

    pub(crate) fn get_mounted_logical_parent(&self, mount: MountId) -> Option<MountRef> {
        self.mounted_logical_parent(mount)
    }

    pub(crate) fn mounted_logical_parent(&self, mount: MountId) -> Option<MountRef> {
        self.with_mount(mount, |mount| mount.logical_parent)
    }

    /// Number of template roots this `mount`'s mount was created with.
    /// Anchor lookups that walk a view's `template.roots()` may iterate
    /// beyond what the mount actually has — e.g. when the view was a clone
    /// whose template grew between renders — and the underlying `root_ids`
    /// would panic on out-of-range indexing.
    pub(crate) fn mounted_root_count(&self, mount: MountId) -> usize {
        self.with_mount(mount, |mount| mount.root_ids.len())
    }

    /// Number of dynamic-node slots this `mount`'s mount was created with.
    /// Same guard rail as [`Self::mounted_root_count`], but for
    /// `mounted_dynamic_nodes`.
    pub(crate) fn mounted_dyn_node_count(&self, mount: MountId) -> usize {
        self.with_mount(mount, |mount| mount.mounted_dynamic_nodes.len())
    }

    pub(crate) fn get_mounted_dynamic_node_slot(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> MountedDynamicNodeSlot {
        self.with_mounted_dynamic_node_slot(mount, dyn_node_idx, |slot| *slot)
    }

    pub(crate) fn set_mounted_dynamic_node_slot(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        value: MountedDynamicNodeSlot,
    ) {
        self.with_mounted_dynamic_node_slot_mut(mount, dyn_node_idx, |slot| *slot = value);
    }

    pub(crate) fn clear_mounted_dynamic_node_slot(&self, mount: MountId, dyn_node_idx: usize) {
        self.set_mounted_dynamic_node_slot(mount, dyn_node_idx, MountedDynamicNodeSlot::Empty);
    }

    pub(crate) fn mounted_dynamic_text_node(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> Option<MountedElementId> {
        self.get_mounted_dynamic_node_slot(mount, dyn_node_idx)
            .text()
    }

    pub(crate) fn unchecked_mounted_dynamic_text_node(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> MountedElementId {
        self.mounted_dynamic_text_node(mount, dyn_node_idx)
            .expect("dynamic text node slot should be mounted")
    }

    pub(crate) fn set_mounted_dynamic_text_node(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        value: MountedElementId,
    ) {
        self.set_mounted_dynamic_node_slot(
            mount,
            dyn_node_idx,
            MountedDynamicNodeSlot::Text(value),
        );
    }

    pub(crate) fn clear_mounted_dynamic_text_node(&self, mount: MountId, dyn_node_idx: usize) {
        self.clear_mounted_dynamic_node_slot(mount, dyn_node_idx);
    }

    pub(crate) fn mounted_dynamic_component_scope(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> Option<ScopeId> {
        self.get_mounted_dynamic_node_slot(mount, dyn_node_idx)
            .component_scope()
    }

    pub(crate) fn mounted_dynamic_component_root_mount(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> Option<MountId> {
        self.get_mounted_dynamic_node_slot(mount, dyn_node_idx)
            .component_root_mount()
    }

    pub(crate) fn unchecked_mounted_dynamic_component_scope(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> ScopeId {
        self.mounted_dynamic_component_scope(mount, dyn_node_idx)
            .expect("dynamic component scope slot should be mounted")
    }

    pub(crate) fn set_mounted_dynamic_component_scope(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        value: ScopeId,
    ) {
        self.with_mounted_dynamic_node_slot_mut(mount, dyn_node_idx, |slot| {
            slot.set_component_scope(value);
        });
    }

    pub(crate) fn set_mounted_dynamic_component_root_mount(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        value: Option<MountId>,
    ) {
        self.with_mounted_dynamic_node_slot_mut(mount, dyn_node_idx, |slot| {
            slot.set_component_root_mount(value);
        });
    }

    pub(crate) fn clear_mounted_dynamic_component_scope(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) {
        self.clear_mounted_dynamic_node_slot(mount, dyn_node_idx);
    }

    pub(crate) fn mounted_dyn_attr(
        &self,
        mount: MountId,
        dyn_attr_idx: usize,
    ) -> Option<MountedElementId> {
        self.mounted_element_slot(mount, dyn_attr_idx, |mount| {
            mount.mounted_attributes.as_ref()
        })
    }

    pub(crate) fn unchecked_mounted_dyn_attr(
        &self,
        mount: MountId,
        dyn_attr_idx: usize,
    ) -> MountedElementId {
        self.mounted_dyn_attr(mount, dyn_attr_idx)
            .expect("dynamic attribute slot should be mounted")
    }

    pub(crate) fn set_mounted_dyn_attr(
        &self,
        mount: MountId,
        dyn_attr_idx: usize,
        value: MountedElementId,
    ) {
        self.set_mounted_element_slot(mount, dyn_attr_idx, Some(value), |mount| {
            mount.mounted_attributes.as_mut()
        });
    }

    pub(crate) fn clear_mounted_dyn_attr(&self, mount: MountId, dyn_attr_idx: usize) {
        self.set_mounted_element_slot(mount, dyn_attr_idx, None, |mount| {
            mount.mounted_attributes.as_mut()
        });
    }

    pub(crate) fn mounted_root_node(
        &self,
        mount: MountId,
        root_idx: usize,
    ) -> Option<MountedElementId> {
        self.mounted_element_slot(mount, root_idx, |mount| mount.root_ids.as_ref())
    }

    pub(crate) fn unchecked_mounted_root_node(
        &self,
        mount: MountId,
        root_idx: usize,
    ) -> MountedElementId {
        self.mounted_root_node(mount, root_idx)
            .expect("root node slot should be mounted")
    }

    pub(crate) fn set_mounted_root_node(
        &self,
        mount: MountId,
        root_idx: usize,
        value: MountedElementId,
    ) {
        self.set_mounted_element_slot(mount, root_idx, Some(value), |mount| {
            mount.root_ids.as_mut()
        });
    }

    pub(crate) fn clear_mounted_root_node(&self, mount: MountId, root_idx: usize) {
        self.set_mounted_element_slot(mount, root_idx, None, |mount| mount.root_ids.as_mut());
    }

    pub(crate) fn current_mounted_view(&self, mount: MountId) -> Option<VNode> {
        // Hand out a deep clone so placement lookups that descend into the
        // returned tree can't observe descendant mount cells being mutated
        // by a sibling diff's `claim_mount`.
        self.runtime
            .mounts
            .borrow()
            .get(mount.0)
            .map(|mount| mount.node.deep_clone_preserving_mounts())
    }

    pub(crate) fn set_mount_mode(&self, mount: MountId, mode: RenderMode) {
        self.with_mount_mut(mount, |mount| mount.mode = mode);
    }

    pub(crate) fn mount_should_render(&self, mount: MountId) -> bool {
        self.runtime
            .mounts
            .borrow()
            .get(mount.0)
            .is_none_or(|mount| mount.mode == RenderMode::Foreground)
    }

    pub(crate) fn claim_mount(&self, old: &VNode, new: &VNode) -> MountId {
        let mount = old.take_mounted_id();
        new.set_mounted_id(mount);
        mount
    }

    pub(crate) fn commit_mount(&self, mount: MountId, node: &VNode) {
        // Every caller commits work on a `mount` that's just been claimed via
        // `claim_mount` or freshly allocated in `create_with_parents` —
        // both produce live `MountId`s.
        // Deep-clone so the committed snapshot owns its own per-vnode
        // raw mount slots. A subsequent diff that calls
        // `claim_mount` on descendant `old` vnodes would otherwise
        // mutate the shared `Rc<VNodeInner>` here too, and placement lookups
        // that walk `mount.node` would see those descendants as unmounted.
        self.runtime.mounts.borrow_mut()[mount.0].node = node.deep_clone_preserving_mounts();
    }

    pub(crate) fn replace_mounted_component_root_mount(
        &self,
        old_root_mount: MountId,
        new_root_mount: MountId,
    ) {
        if old_root_mount == new_root_mount {
            return;
        }

        let mut mounts = self.runtime.mounts.borrow_mut();
        for (_, mount) in mounts.iter_mut() {
            for slot in mount.mounted_dynamic_nodes.iter_mut() {
                let MountedDynamicNodeSlot::Component { root_mount, .. } = slot else {
                    continue;
                };
                if *root_mount == Some(old_root_mount) {
                    *root_mount = Some(new_root_mount);
                }
            }
        }
    }

    fn with_mount<R>(&self, mount: MountId, with_mount: impl FnOnce(&Mount) -> R) -> R {
        self.runtime
            .mounts
            .borrow()
            .get(mount.0)
            .map(with_mount)
            .expect("mounted mount record should exist")
    }

    fn with_mount_mut<R>(&self, mount: MountId, with_mount: impl FnOnce(&mut Mount) -> R) -> R {
        self.runtime
            .mounts
            .borrow_mut()
            .get_mut(mount.0)
            .map(with_mount)
            .expect("mounted mount record should exist")
    }

    fn with_mounted_dynamic_node_slot<R>(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        with_slot: impl FnOnce(&MountedDynamicNodeSlot) -> R,
    ) -> R {
        self.with_mount(mount, |mount| {
            with_slot(&mount.mounted_dynamic_nodes[dyn_node_idx])
        })
    }

    fn with_mounted_dynamic_node_slot_mut<R>(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        with_slot: impl FnOnce(&mut MountedDynamicNodeSlot) -> R,
    ) -> R {
        self.with_mount_mut(mount, |mount| {
            with_slot(&mut mount.mounted_dynamic_nodes[dyn_node_idx])
        })
    }

    fn mounted_element_slot(
        &self,
        mount: MountId,
        idx: usize,
        slots: impl FnOnce(&Mount) -> &[Option<MountedElementId>],
    ) -> Option<MountedElementId> {
        self.with_mount(mount, |mount| slots(mount)[idx])
    }

    fn set_mounted_element_slot(
        &self,
        mount: MountId,
        idx: usize,
        value: Option<MountedElementId>,
        slots: impl FnOnce(&mut Mount) -> &mut [Option<MountedElementId>],
    ) {
        self.with_mount_mut(mount, |mount| {
            slots(mount)[idx] = value;
        });
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
        let root_mount = root.unchecked_mounted_id();
        // Deep-clone on the way in so the stored root has its own
        // `VNodeInner`. Subsequent diffs against this branch can take per-slot
        // mounts via `claim_mount` without modifying any raw mount slots
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
