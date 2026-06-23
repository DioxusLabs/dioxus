//! Mount-table bookkeeping for committed vnodes.
//!
//! Invariants maintained here:
//! - A live `MountId` owns one committed `VNode`, one render parent, one logical parent, and one
//!   render target.
//! - Anchor slots and dynamic slots are sized from the committed vnode template and remain stable
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

const PENDING_FRAGMENT_CHILD_MOUNT: MountId = MountId(usize::MAX);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FragmentMountWriter {
    mount: MountId,
    dyn_node_idx: usize,
    start: usize,
    len: usize,
}

#[cfg(debug_assertions)]
impl FragmentMountWriter {
    pub(crate) fn len(self) -> usize {
        self.len
    }
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
        Self { value: index + 1 }
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

    /// Mounted slots for this node in one allocation, laid out as two regions:
    /// `[ anchor slots (anchor_count) | dynamic slots (rest) ]`.
    /// Previously these were three separate `Box<[_]>` - one heap allocation per region, so three
    /// allocs/frees per mounted node. A list of N rows is N mounts, so that tripled the per-node
    /// allocator traffic on both create and removal. One backing slice means one alloc/free.
    slots: Box<[PackedMountedSlot]>,
    anchor_count: u32,

    fragment_child_mounts: Vec<MountId>,
}

impl Mount {
    pub(crate) fn new(
        node: VNode,
        render_parent: Option<MountRef>,
        logical_parent: Option<MountRef>,
        target_id: RenderTargetId,
    ) -> Self {
        // Anchor and dynamic-node slot counts are structural: the anchor array length is the
        // number of template anchors, and the dynamic array length is the number of runtime
        // dynamic nodes. Static roots are stored in their structural root anchors.
        let anchor_count = node.template.anchors().len();
        let dynamic_count = node.dynamic_node_values().len();
        Self {
            render_parent,
            logical_parent,
            target_id,
            node,
            mode: RenderMode::Foreground,
            slots: vec![PackedMountedSlot::empty(); anchor_count + dynamic_count].into(),
            anchor_count: anchor_count as u32,
            fragment_child_mounts: Vec::new(),
        }
    }

    pub(crate) fn mounted_anchor_node(&self, idx: usize) -> Option<MountedElementId> {
        self.anchor_slot(idx).mounted_element()
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

    fn dynamic_offset(&self) -> usize {
        self.anchor_count as usize
    }

    fn dynamic_slot(&self, idx: usize) -> PackedMountedSlot {
        self.slots[self.dynamic_offset() + idx]
    }

    fn dynamic_slot_mut(&mut self, idx: usize) -> &mut PackedMountedSlot {
        let offset = self.dynamic_offset();
        &mut self.slots[offset + idx]
    }

    fn anchor_slot(&self, idx: usize) -> PackedMountedSlot {
        self.slots[idx]
    }

    fn anchor_slot_mut(&mut self, idx: usize) -> &mut PackedMountedSlot {
        &mut self.slots[idx]
    }
}

impl VirtualDom {
    pub(crate) fn create_mount(
        &mut self,
        node: &VNode,
        render_parent: Option<MountRef>,
        logical_parent: Option<MountRef>,
        target_id: RenderTargetId,
    ) -> MountId {
        let mut mounts = self.runtime.mounts.borrow_mut();
        let entry = mounts.vacant_entry();
        let mount = MountId(entry.key());
        entry.insert(Mount::new(
            node.clone(),
            render_parent,
            logical_parent,
            target_id,
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

    pub(crate) fn mounted_anchor_node(
        &self,
        mount: MountId,
        anchor_idx: usize,
    ) -> Option<MountedElementId> {
        self.with_mount(mount, |mount| {
            mount.anchor_slot(anchor_idx).mounted_element()
        })
    }

    #[track_caller]
    pub(crate) fn unchecked_mounted_anchor_node(
        &self,
        mount: MountId,
        anchor_idx: usize,
    ) -> MountedElementId {
        self.mounted_anchor_node(mount, anchor_idx)
            .expect("anchor slot")
    }

    pub(crate) fn set_mounted_anchor_node(
        &self,
        mount: MountId,
        anchor_idx: usize,
        value: MountedElementId,
    ) {
        self.with_mount_mut(mount, |mount| {
            *mount.anchor_slot_mut(anchor_idx) = PackedMountedSlot::from_mounted_element(value);
        });
    }

    pub(crate) fn clear_mounted_anchor_node(&self, mount: MountId, anchor_idx: usize) {
        self.with_mount_mut(mount, |mount| {
            *mount.anchor_slot_mut(anchor_idx) = PackedMountedSlot::empty();
        });
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

    pub(crate) fn begin_mounted_fragment_children(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        len: usize,
    ) -> FragmentMountWriter {
        self.with_mount_mut(mount, |mount_state| {
            let start = mount_state.fragment_child_mounts.len();
            mount_state
                .fragment_child_mounts
                .resize(start + len, PENDING_FRAGMENT_CHILD_MOUNT);
            FragmentMountWriter {
                mount,
                dyn_node_idx,
                start,
                len,
            }
        })
    }

    pub(crate) fn set_mounted_fragment_child(
        &self,
        writer: FragmentMountWriter,
        idx: usize,
        child: MountId,
    ) {
        debug_assert_ne!(
            child, PENDING_FRAGMENT_CHILD_MOUNT,
            "pending fragment sentinel cannot be a live child mount"
        );
        debug_assert!(
            idx < writer.len,
            "fragment child write index must fit the pending range"
        );
        self.with_mount_mut(writer.mount, |mount| {
            mount.fragment_child_mounts[writer.start + idx] = child;
        });
    }

    pub(crate) fn commit_mounted_fragment_children(&self, writer: FragmentMountWriter) {
        self.with_mount_mut(writer.mount, |mount| {
            if writer.len == 0 {
                *mount.dynamic_slot_mut(writer.dyn_node_idx) = PackedMountedSlot::empty();
                return;
            }

            let range = writer.start..writer.start + writer.len;
            debug_assert!(
                range.end <= mount.fragment_child_mounts.len(),
                "pending fragment range must fit fragment storage"
            );
            debug_assert!(
                mount.fragment_child_mounts[range]
                    .iter()
                    .all(|mount| { *mount != PENDING_FRAGMENT_CHILD_MOUNT }),
                "pending fragment range must be fully written before commit"
            );
            *mount.dynamic_slot_mut(writer.dyn_node_idx) =
                PackedMountedSlot::from_slot(MountedDynamicNodeSlot::Fragment(writer.start));
        });
    }

    pub(crate) fn mounted_fragment_children(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        len: usize,
    ) -> Vec<MountId> {
        self.try_with_mounted_fragment_children(mount, dyn_node_idx, len, |children| {
            children.to_vec()
        })
        .expect("non-empty mounted fragment should have a child range")
    }

    pub(crate) fn try_with_mounted_fragment_children<R>(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        len: usize,
        with_children: impl FnOnce(&[MountId]) -> R,
    ) -> Option<R> {
        self.with_mount(mount, |mount| {
            if len == 0 {
                return Some(with_children(&[]));
            }
            let start = mount.dynamic_slot(dyn_node_idx).fragment_start()?;
            mount
                .fragment_child_mounts
                .get(start..start + len)
                .map(with_children)
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

    pub(crate) fn mounted_dynamic_component_root_mount(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
    ) -> Option<MountId> {
        let scope = self.mounted_dynamic_component_scope(mount, dyn_node_idx)?;
        self.runtime.try_get_state(scope)?.root_mount()
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
    for anchor in node.dynamic_anchors() {
        for slot in anchor.nodes() {
            let idx = slot.index();
            let DynamicNode::Fragment(nodes) = &*slot else {
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
/// visible. The root `VNode` is the render output we diff, and the branch also
/// records the root mount identity so the boundary state is tied to retained
/// mount ownership.
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
