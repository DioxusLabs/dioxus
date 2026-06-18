//! Mount-table bookkeeping for committed vnodes.
//!
//! Invariants maintained here:
//! - A live `MountId` owns one committed `VNode`, one render parent, one logical parent, and one
//!   render target.
//! - Root slots and dynamic slots are sized from the committed vnode template and remain stable
//!   until `commit_mount`.
//! - Non-empty fragment dynamic slots point at an exact contiguous range in
//!   `fragment_child_mounts`; empty fragments store an empty slot.
//! - Diff internals must use `mounted_fragment_children_exact` when vnode shape says a fragment has
//!   children. The permissive `mounted_fragment_children` accessor is for public inspection paths
//!   where "not a fragment" should produce an empty list.

use crate::{
    DynamicNode, RenderTargetId, ScopeId, VNode,
    arena::{MountId, MountRef, MountedDynamicNodeSlot, MountedElementId},
    virtual_dom::VirtualDom,
};

/// Whether a mount is allowed to write renderer mutations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RenderMode {
    Foreground,
    Background,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PackedMountedSlot {
    value: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MountedDynamicNodeSlotSnapshot {
    value: usize,
}

impl PackedMountedSlot {
    const fn empty() -> Self {
        Self { value: 0 }
    }

    fn from_mounted_element(id: MountedElementId) -> Self {
        Self::from_non_zero(id.index())
    }

    fn from_slot(slot: MountedDynamicNodeSlot) -> Self {
        match slot {
            MountedDynamicNodeSlot::Empty => Self::empty(),
            MountedDynamicNodeSlot::Text(id) => Self::from_non_zero(id.index()),
            MountedDynamicNodeSlot::Component(scope) => Self::from_index(scope.index()),
            MountedDynamicNodeSlot::Fragment(start) => Self::from_index(start),
        }
    }

    fn from_index(index: usize) -> Self {
        Self {
            value: index.checked_add(1).expect("slot overflow"),
        }
    }

    fn from_non_zero(index: usize) -> Self {
        debug_assert_ne!(index, 0);
        Self { value: index }
    }

    fn snapshot(self) -> MountedDynamicNodeSlotSnapshot {
        MountedDynamicNodeSlotSnapshot { value: self.value }
    }

    fn from_snapshot(snapshot: MountedDynamicNodeSlotSnapshot) -> Self {
        Self {
            value: snapshot.value,
        }
    }

    fn mounted_element(self) -> Option<MountedElementId> {
        (self.value != 0).then(|| MountedElementId::from_index_unchecked(self.value))
    }

    fn text(self) -> Option<MountedElementId> {
        self.mounted_element()
    }

    fn component_scope(self) -> Option<ScopeId> {
        self.value.checked_sub(1).map(ScopeId::new)
    }

    fn fragment_start(self) -> Option<usize> {
        self.value.checked_sub(1)
    }

    fn set_component_scope(&mut self, scope: ScopeId) {
        *self = Self::from_slot(MountedDynamicNodeSlot::Component(scope));
    }
}

pub(crate) struct Mount {
    render_parent: Option<MountRef>,

    logical_parent: Option<MountRef>,

    target_id: RenderTargetId,

    node: VNode,

    mode: RenderMode,

    root_count: usize,

    mounted_slots: Box<[PackedMountedSlot]>,

    fragment_child_mounts: Vec<MountId>,
}

impl Mount {
    pub(crate) fn new(
        node: VNode,
        render_parent: Option<MountRef>,
        logical_parent: Option<MountRef>,
        target_id: RenderTargetId,
        root_count: usize,
        dynamic_count: usize,
    ) -> Self {
        Self {
            render_parent,
            logical_parent,
            target_id,
            node,
            mode: RenderMode::Foreground,
            root_count,
            mounted_slots: vec![PackedMountedSlot::empty(); root_count + dynamic_count].into(),
            fragment_child_mounts: Vec::new(),
        }
    }

    pub(crate) fn mounted_attribute(&self, idx: usize) -> Option<MountedElementId> {
        self.dynamic_slot(idx).mounted_element()
    }

    pub(crate) fn non_empty_fragment_children(&self, idx: usize, len: usize) -> &[MountId] {
        debug_assert!(len > 0, "fragment child slice accessor requires children");
        let start = self
            .dynamic_slot(idx)
            .fragment_start()
            .expect("fragment children");
        &self.fragment_child_mounts[start..start + len]
    }

    pub(crate) fn component_scope(&self, idx: usize) -> Option<ScopeId> {
        self.dynamic_slot(idx).component_scope()
    }

    pub(crate) fn logical_parent(&self) -> Option<MountRef> {
        self.logical_parent
    }

    pub(crate) fn node(&self) -> &VNode {
        &self.node
    }

    fn dynamic_slot(&self, idx: usize) -> PackedMountedSlot {
        self.mounted_slots[self.root_count + idx]
    }

    fn dynamic_slot_mut(&mut self, idx: usize) -> &mut PackedMountedSlot {
        &mut self.mounted_slots[self.root_count + idx]
    }

    fn root_slot(&self, idx: usize) -> PackedMountedSlot {
        self.mounted_slots[idx]
    }

    fn root_slot_mut(&mut self, idx: usize) -> &mut PackedMountedSlot {
        &mut self.mounted_slots[idx]
    }
}

macro_rules! mounted_element_accessors {
    ($mounted:ident, $unchecked:ident, $set:ident, $clear:ident, $idx:ident, $get:expr, $set_slot:expr, $expect:literal) => {
        pub(crate) fn $mounted(&self, mount: MountId, $idx: usize) -> Option<MountedElementId> {
            self.with_mount(mount, $get)
        }

        #[track_caller]
        pub(crate) fn $unchecked(&self, mount: MountId, $idx: usize) -> MountedElementId {
            self.$mounted(mount, $idx).expect($expect)
        }

        pub(crate) fn $set(&self, mount: MountId, $idx: usize, value: MountedElementId) {
            self.with_mount_mut(mount, |mount| {
                ($set_slot)(mount, $idx, PackedMountedSlot::from_mounted_element(value))
            });
        }

        pub(crate) fn $clear(&self, mount: MountId, $idx: usize) {
            self.with_mount_mut(mount, |mount| {
                ($set_slot)(mount, $idx, PackedMountedSlot::empty())
            });
        }
    };
}

impl VirtualDom {
    pub(crate) fn create_mount(
        &mut self,
        node: &VNode,
        render_parent: Option<MountRef>,
        logical_parent: Option<MountRef>,
        target_id: RenderTargetId,
        root_count: usize,
        dynamic_count: usize,
    ) -> MountId {
        let mut mounts = self.runtime.mounts.borrow_mut();
        let entry = mounts.vacant_entry();
        let mount = MountId(entry.key());
        entry.insert(Mount::new(
            node.clone(),
            render_parent,
            logical_parent,
            target_id,
            root_count,
            dynamic_count,
        ));

        mount
    }

    pub(crate) fn reuse_mount(
        &mut self,
        mount: MountId,
        render_parent: Option<MountRef>,
        logical_parent: Option<MountRef>,
        target_id: RenderTargetId,
    ) {
        self.with_mount_mut(mount, |mount| {
            mount.render_parent = render_parent;
            mount.logical_parent = logical_parent;
            mount.target_id = target_id;
        });
    }

    pub(crate) fn remove_mount(&mut self, mount: MountId) {
        self.runtime.mounts.borrow_mut().remove(mount.0);
    }

    pub(crate) fn mount_target_id(&self, mount: MountId) -> RenderTargetId {
        self.with_mount(mount, |mount| mount.target_id)
    }

    pub(crate) fn mounted_render_parent(&self, mount: MountId) -> Option<MountRef> {
        self.with_mount(mount, |mount| mount.render_parent)
    }

    pub(crate) fn mounted_logical_parent(&self, mount: MountId) -> Option<MountRef> {
        self.with_mount(mount, |mount| mount.logical_parent)
    }

    pub(crate) fn mounted_root_count(&self, mount: MountId) -> usize {
        self.with_mount(mount, |mount| mount.root_count)
    }

    pub(crate) fn mounted_dyn_node_count(&self, mount: MountId) -> usize {
        self.with_mount(mount, |mount| mount.mounted_slots.len() - mount.root_count)
    }

    pub(crate) fn mounted_dynamic_node_slot_snapshot(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> MountedDynamicNodeSlotSnapshot {
        self.with_mount(mount, |mount| mount.dynamic_slot(dyn_node_idx).snapshot())
    }

    pub(crate) fn restore_mounted_dynamic_node_slot(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        value: MountedDynamicNodeSlotSnapshot,
    ) {
        self.set_packed_mounted_dynamic_node_slot(
            mount,
            dyn_node_idx,
            PackedMountedSlot::from_snapshot(value),
        );
    }

    pub(crate) fn set_mounted_dynamic_node_slot(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        value: MountedDynamicNodeSlot,
    ) {
        self.set_packed_mounted_dynamic_node_slot(
            mount,
            dyn_node_idx,
            PackedMountedSlot::from_slot(value),
        );
    }

    fn set_packed_mounted_dynamic_node_slot(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        value: PackedMountedSlot,
    ) {
        self.with_mount_mut(mount, |mount| {
            *mount.dynamic_slot_mut(dyn_node_idx) = value;
        });
    }

    pub(crate) fn clear_mounted_dynamic_node_slot(&self, mount: MountId, dyn_node_idx: usize) {
        self.set_mounted_dynamic_node_slot(mount, dyn_node_idx, MountedDynamicNodeSlot::Empty);
    }

    pub(crate) fn mounted_dynamic_text_node(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> Option<MountedElementId> {
        self.with_mount(mount, |mount| mount.dynamic_slot(dyn_node_idx).text())
    }

    pub(crate) fn unchecked_mounted_dynamic_text_node(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> MountedElementId {
        self.mounted_dynamic_text_node(mount, dyn_node_idx)
            .expect("text slot")
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

    pub(crate) fn clear_mounted_fragment_children(&self, mount: MountId, dyn_node_idx: usize) {
        self.with_mount_mut(mount, |mount| {
            *mount.dynamic_slot_mut(dyn_node_idx) = PackedMountedSlot::empty();
        });
    }

    pub(crate) fn set_mounted_fragment_children_vec(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        children: Vec<MountId>,
    ) {
        self.with_mount_mut(mount, |mount| {
            if children.is_empty() {
                *mount.dynamic_slot_mut(dyn_node_idx) = PackedMountedSlot::empty();
                return;
            }

            let start = mount.fragment_child_mounts.len();
            if mount.fragment_child_mounts.is_empty() {
                mount.fragment_child_mounts = children;
            } else {
                mount.fragment_child_mounts.extend(children);
            }
            *mount.dynamic_slot_mut(dyn_node_idx) =
                PackedMountedSlot::from_slot(MountedDynamicNodeSlot::Fragment(start));
        });
    }

    pub(crate) fn mounted_fragment_children(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        len: usize,
    ) -> Vec<MountId> {
        self.with_mount(mount, |mount| {
            if len == 0 {
                return Vec::new();
            }
            let start = mount
                .dynamic_slot(dyn_node_idx)
                .fragment_start()
                .expect("non-empty mounted fragment should have a child range");
            mount.fragment_child_mounts[start..start + len].to_vec()
        })
    }

    /// Return the fragment child mounts for a vnode fragment whose shape is known.
    ///
    /// Invariant: `mount` is live, `dyn_node_idx` is a fragment slot, and `len` is the current
    /// fragment child count from the vnode that owns the slot.
    pub(crate) fn mounted_fragment_children_exact(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        len: usize,
    ) -> Vec<MountId> {
        let children = self.mounted_fragment_children(mount, dyn_node_idx, len);
        assert!(children.len() == len, "fragment slots");
        children
    }

    pub(crate) fn mounted_dynamic_component_scope(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> Option<ScopeId> {
        self.with_mount(mount, |mount| {
            mount.dynamic_slot(dyn_node_idx).component_scope()
        })
    }

    pub(crate) fn unchecked_mounted_dynamic_component_root_mount(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> MountId {
        let scope = self.unchecked_mounted_dynamic_component_scope(mount, dyn_node_idx);
        self.runtime
            .get_state(scope)
            .root_mount()
            .expect("component root")
    }

    pub(crate) fn unchecked_mounted_dynamic_component_scope(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> ScopeId {
        self.mounted_dynamic_component_scope(mount, dyn_node_idx)
            .expect("component slot")
    }

    pub(crate) fn set_mounted_dynamic_component_scope(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        value: ScopeId,
    ) {
        self.with_mount_mut(mount, |mount| {
            mount
                .dynamic_slot_mut(dyn_node_idx)
                .set_component_scope(value);
        });
    }

    mounted_element_accessors!(
        mounted_dyn_attr,
        unchecked_mounted_dyn_attr,
        set_mounted_dyn_attr,
        clear_mounted_dyn_attr,
        dyn_attr_idx,
        |mount: &Mount| mount.dynamic_slot(dyn_attr_idx).mounted_element(),
        |mount: &mut Mount, idx, value| *mount.dynamic_slot_mut(idx) = value,
        "attr slot"
    );

    mounted_element_accessors!(
        mounted_root_node,
        unchecked_mounted_root_node,
        set_mounted_root_node,
        clear_mounted_root_node,
        root_idx,
        |mount: &Mount| mount.root_slot(root_idx).mounted_element(),
        |mount: &mut Mount, idx, value| *mount.root_slot_mut(idx) = value,
        "root slot"
    );

    pub(crate) fn current_mounted_view(&self, mount: MountId) -> Option<VNode> {
        self.runtime
            .mounts
            .borrow()
            .get(mount.0)
            .map(|mount| mount.node.clone())
    }

    pub(crate) fn set_mount_mode(&self, mount: MountId, mode: RenderMode) {
        self.with_mount_mut(mount, |mount| mount.mode = mode);
    }

    pub(crate) fn mount_should_render(&self, mount: MountId) -> bool {
        self.with_mount(mount, |mount| mount.mode == RenderMode::Foreground)
    }

    /// Commit the new vnode for a mount after its roots/dynamic slots have been updated.
    ///
    /// Invariant: fragment child storage may contain stale ranges accumulated during the diff; this
    /// compacts storage so every committed non-empty fragment slot owns exactly its current range.
    pub(crate) fn commit_mount(&self, mount: MountId, node: &VNode) {
        let mut mounts = self.runtime.mounts.borrow_mut();
        let mount_state = &mut mounts[mount.0];
        mount_state.node = node.clone();
        compact_fragment_child_mounts(mount_state, node);
        #[cfg(all(debug_assertions, not(coverage_nightly)))]
        assert_committed_fragment_slots(mount_state, node);
    }

    pub(crate) fn replace_mounted_component_root_mount(
        &self,
        old_root_mount: MountId,
        new_root_mount: MountId,
    ) {
        if old_root_mount == new_root_mount {
            return;
        }

        for scope in self.runtime.scope_states.borrow().iter().flatten() {
            if scope.root_mount() == Some(old_root_mount) {
                scope.set_root_mount(Some(new_root_mount));
            }
        }
    }

    fn with_mount<R>(&self, mount: MountId, with_mount: impl FnOnce(&Mount) -> R) -> R {
        self.runtime
            .mounts
            .borrow()
            .get(mount.0)
            .map(with_mount)
            .expect("mount")
    }

    fn with_mount_mut<R>(&self, mount: MountId, with_mount: impl FnOnce(&mut Mount) -> R) -> R {
        self.runtime
            .mounts
            .borrow_mut()
            .get_mut(mount.0)
            .map(with_mount)
            .expect("mount")
    }
}

fn compact_fragment_child_mounts(mount: &mut Mount, node: &VNode) {
    if mount.fragment_child_mounts.is_empty() {
        return;
    }

    let old_children = std::mem::take(&mut mount.fragment_child_mounts);
    for (idx, value) in node.dynamic_values.iter().enumerate() {
        let Some(DynamicNode::Fragment(nodes)) = value.as_node() else {
            continue;
        };

        if nodes.is_empty() {
            *mount.dynamic_slot_mut(idx) = PackedMountedSlot::empty();
            continue;
        }

        let start = mount
            .dynamic_slot(idx)
            .fragment_start()
            .expect("fragment children");
        let range = start..start + nodes.len();
        let new_start = mount.fragment_child_mounts.len();
        mount
            .fragment_child_mounts
            .extend_from_slice(&old_children[range]);
        *mount.dynamic_slot_mut(idx) =
            PackedMountedSlot::from_slot(MountedDynamicNodeSlot::Fragment(new_start));
    }
}

#[cfg(all(debug_assertions, not(coverage_nightly)))]
fn assert_committed_fragment_slots(mount: &Mount, node: &VNode) {
    for (idx, value) in node.dynamic_values.iter().enumerate() {
        let Some(DynamicNode::Fragment(nodes)) = value.as_node() else {
            continue;
        };
        if nodes.is_empty() {
            debug_assert!(
                mount.dynamic_slot(idx).fragment_start().is_none(),
                "empty fragment dynamic slots must be empty after commit"
            );
            continue;
        }

        let start = mount
            .dynamic_slot(idx)
            .fragment_start()
            .expect("fragment children");
        debug_assert!(
            start + nodes.len() <= mount.fragment_child_mounts.len(),
            "committed fragment dynamic slot range must stay within mount fragment storage"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::ElementId;
    use std::mem::size_of;

    #[test]
    fn mounted_element_option_is_one_word() {
        assert_eq!(size_of::<Option<MountedElementId>>(), size_of::<usize>());
    }

    #[test]
    fn packed_dynamic_slot_is_one_word() {
        assert_eq!(size_of::<PackedMountedSlot>(), size_of::<usize>());
    }

    #[test]
    fn packed_dynamic_slot_round_trips() {
        let text = MountedDynamicNodeSlot::Text(MountedElementId::new_unchecked(ElementId::new(7)));
        let component = MountedDynamicNodeSlot::Component(ScopeId::new(3));

        assert_eq!(
            PackedMountedSlot::from_slot(MountedDynamicNodeSlot::Empty),
            PackedMountedSlot::empty()
        );
        assert_eq!(
            PackedMountedSlot::from_slot(text).text(),
            Some(MountedElementId::new_unchecked(ElementId::new(7)))
        );
        assert_eq!(
            PackedMountedSlot::from_slot(component).component_scope(),
            Some(ScopeId::new(3))
        );

        let fragment = PackedMountedSlot::from_slot(MountedDynamicNodeSlot::Fragment(17));
        assert_eq!(fragment.fragment_start(), Some(17));
    }
}

/// A retained suspense branch.
///
/// Suspense keeps the hidden primary branch alive while the fallback branch is
/// visible. The root `VNode` is still the render output we diff, but the branch
/// also records the root mount identity so the boundary state is explicitly tied
/// to retained mount ownership instead of being just a parked vnode.
#[derive(Clone)]
pub(crate) struct SuspenseBranch {
    root: VNode,
    root_mount: MountId,
}

impl SuspenseBranch {
    pub(crate) fn new(root: VNode, root_mount: MountId) -> Self {
        Self { root, root_mount }
    }

    pub(crate) fn root(&self) -> VNode {
        self.root.clone()
    }

    pub(crate) fn mounted_root(&self) -> crate::MountedVNode<'_> {
        crate::MountedVNode::new(&self.root, self.root_mount)
    }

    pub(crate) fn root_mount(&self) -> MountId {
        self.root_mount
    }

    pub(crate) fn into_root(self) -> VNode {
        self.root
    }
}
