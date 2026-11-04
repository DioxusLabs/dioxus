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
    DynamicNode, DynamicNodeSlot, RenderTargetId, ScopeId, VNode,
    arena::{MountId, MountedDynamicNodeSlot, MountedElementId},
    virtual_dom::VirtualDom,
};

/// Whether a mount is allowed to write renderer mutations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RenderMode {
    Foreground,
    Background,
}

/// This is a union that stores either:
/// - A scope id
/// - A mounted element id
/// - A fragment range
///
/// The discriminate is determined by the dynamic node variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PackedMountedSlot {
    value: usize,
}

const UNWRITTEN_FRAGMENT_CHILD_MOUNT: MountId = MountId(usize::MAX);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FragmentMountWriter {
    mount: MountId,
    anchor_index: usize,
    dyn_node_idx: usize,
    start: usize,
    len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MountedParent {
    parent: MountId,
    anchor_index: usize,
    slot_index: usize,
    kind: MountedParentKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MountedParentKind {
    ComponentRoot,
    /// `child_index` is this child's slot in its parent fragment's committed child list. Placement
    /// indexes the committed (old) child list with it, so it must track the *committed* position:
    /// it is advanced to a child's new index only when the fragment diff commits, never mid-diff.
    FragmentChild {
        child_index: usize,
    },
}

impl MountedParent {
    pub(crate) fn component_root(parent: MountId, anchor_index: usize, slot_index: usize) -> Self {
        Self {
            parent,
            anchor_index,
            slot_index,
            kind: MountedParentKind::ComponentRoot,
        }
    }

    pub(crate) fn fragment_child(
        parent: MountId,
        anchor_index: usize,
        slot_index: usize,
        child_index: usize,
    ) -> Self {
        Self {
            parent,
            anchor_index,
            slot_index,
            kind: MountedParentKind::FragmentChild { child_index },
        }
    }

    pub(crate) fn parent(self) -> MountId {
        self.parent
    }

    pub(crate) fn anchor_index(self) -> usize {
        self.anchor_index
    }

    pub(crate) fn slot<'a>(self, vnode: &'a VNode) -> Option<DynamicNodeSlot<'a>> {
        vnode.dynamic_node_slot(self.anchor_index, self.slot_index)
    }

    pub(crate) fn kind(self) -> MountedParentKind {
        self.kind
    }
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
        Self::from_index(id.index())
    }

    fn from_slot(slot: MountedDynamicNodeSlot) -> Self {
        match slot {
            MountedDynamicNodeSlot::Empty => Self::empty(),
            MountedDynamicNodeSlot::Text(id) => Self::from_index(id.index()),
            MountedDynamicNodeSlot::Component(scope) => Self::from_index(scope.index()),
            MountedDynamicNodeSlot::Fragment(start) => Self::from_index(start),
        }
    }

    fn from_index(index: usize) -> Self {
        Self { value: index + 1 }
    }

    fn mounted_element(self) -> Option<MountedElementId> {
        self.value.checked_sub(1).map(MountedElementId::from_index)
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
    render_parent: Option<MountedParent>,

    logical_parent: Option<MountId>,

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
        logical_parent: Option<MountId>,
        target_id: RenderTargetId,
    ) -> Self {
        // Anchor and dynamic-node slot counts are structural: the anchor array length is the
        // number of template anchors, and the dynamic array length is the number of runtime
        // dynamic nodes. Static roots are stored in their structural root anchors.
        let anchor_count = node.template().anchors().len();
        let dynamic_count = node.dynamic_node_values().len();
        Self {
            render_parent: None,
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

    pub(crate) fn logical_parent(&self) -> Option<MountId> {
        self.logical_parent
    }

    pub(crate) fn render_parent(&self) -> Option<MountedParent> {
        self.render_parent
    }

    pub(crate) fn node(&self) -> &VNode {
        &self.node
    }

    fn dynamic_node_anchor_index(&self, dyn_node_idx: usize) -> usize {
        self.node
            .dynamic_anchors()
            .find(|anchor| anchor.nodes().any(|slot| slot.index() == dyn_node_idx))
            .map(|anchor| anchor.anchor_index())
            .expect("dynamic node should belong to an anchor")
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
        logical_parent: Option<MountId>,
        target_id: RenderTargetId,
    ) -> MountId {
        let mut mounts = self.runtime.mounts.borrow_mut();
        let entry = mounts.vacant_entry();
        let mount = MountId(entry.key());
        entry.insert(Mount::new(node.clone(), logical_parent, target_id));

        mount
    }

    /// Reuse an existing mount's allocation (keeping its element ids and child
    /// component/scope slots) by re-parenting it. Paired with `finish_create` to
    /// re-emit a background-created subtree to the foreground writer without
    /// allocating fresh scopes.
    pub(crate) fn reuse_mount(
        &mut self,
        mount: MountId,
        render_parent: Option<MountId>,
        logical_parent: Option<MountId>,
        target_id: RenderTargetId,
    ) {
        self.with_mount_mut(mount, |mount| {
            mount.render_parent = match (mount.render_parent, render_parent) {
                (Some(current), Some(parent)) if current.parent() == parent => Some(current),
                _ => None,
            };
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

    pub(crate) fn mounted_render_parent(&self, mount: MountId) -> Option<MountedParent> {
        self.with_mount(mount, |mount| mount.render_parent())
    }

    pub(crate) fn set_mounted_render_parent(&self, mount: MountId, render_parent: MountedParent) {
        self.with_mount_mut(mount, |mount| {
            mount.render_parent = Some(render_parent);
        });
    }

    pub(crate) fn mounted_logical_parent(&self, mount: MountId) -> Option<MountId> {
        self.with_mount(mount, |mount| mount.logical_parent)
    }

    pub(crate) fn copy_render_parent_slot(&self, from: MountId, to: MountId) {
        let mut mounts = self.runtime.mounts.borrow_mut();
        let render_parent = mounts[from.0].render_parent();
        mounts[to.0].render_parent = render_parent;
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
    ) -> PackedMountedSlot {
        self.with_mount(mount, |mount| mount.dynamic_slot(dyn_node_idx))
    }

    pub(crate) fn restore_mounted_dynamic_node_slot(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        value: PackedMountedSlot,
    ) {
        self.set_packed_mounted_dynamic_node_slot(mount, dyn_node_idx, value);
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
            let anchor_index = mount_state.dynamic_node_anchor_index(dyn_node_idx);
            mount_state
                .fragment_child_mounts
                .resize(start + len, UNWRITTEN_FRAGMENT_CHILD_MOUNT);
            FragmentMountWriter {
                mount,
                anchor_index,
                dyn_node_idx,
                start,
                len,
            }
        })
    }

    /// Write `child` into fragment slot `idx` and home its render parent at that slot.
    ///
    /// `idx` becomes the child's committed `child_index`, so this is only correct once the slot is
    /// the child's final position (the create path and the post-diff commit). During a diff, use
    /// [`Self::stage_mounted_fragment_child`] so placement keeps reading the old committed index.
    pub(crate) fn set_mounted_fragment_child(
        &self,
        writer: FragmentMountWriter,
        idx: usize,
        child: MountId,
    ) {
        let render_parent = MountedParent::fragment_child(
            writer.mount,
            writer.anchor_index,
            writer.dyn_node_idx,
            idx,
        );
        let mut mounts = self.stage_mounted_fragment_child_inner(writer, idx, child);
        mounts[child.0].render_parent = Some(render_parent);
    }

    /// Write `child` into the pending fragment slot without re-homing its render parent.
    ///
    /// Used while a fragment diff is in flight: the child's render parent must keep pointing at its
    /// old committed index (which placement reads against the old child list) until the whole diff
    /// commits and [`Self::set_mounted_fragment_child`] advances it to the new index.
    pub(crate) fn stage_mounted_fragment_child(
        &self,
        writer: FragmentMountWriter,
        idx: usize,
        child: MountId,
    ) {
        let _ = self.stage_mounted_fragment_child_inner(writer, idx, child);
    }

    fn stage_mounted_fragment_child_inner(
        &self,
        writer: FragmentMountWriter,
        idx: usize,
        child: MountId,
    ) -> std::cell::RefMut<'_, slab::Slab<Mount>> {
        debug_assert_ne!(
            child, UNWRITTEN_FRAGMENT_CHILD_MOUNT,
            "unwritten fragment child sentinel cannot be a live child mount"
        );
        debug_assert!(
            idx < writer.len,
            "fragment child write index must fit the pending range"
        );
        let mut mounts = self.runtime.mounts.borrow_mut();
        mounts[writer.mount.0].fragment_child_mounts[writer.start + idx] = child;
        mounts
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
                "fragment child writer range must fit fragment storage"
            );
            debug_assert!(
                mount.fragment_child_mounts[range]
                    .iter()
                    .all(|mount| { *mount != UNWRITTEN_FRAGMENT_CHILD_MOUNT }),
                "fragment child writer range must be fully written before commit"
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

    pub(crate) fn set_component_root_render_parent(
        &self,
        mount: MountId,
        dyn_node_idx: usize,
        root_mount: MountId,
    ) {
        let anchor_index =
            self.with_mount(mount, |mount| mount.dynamic_node_anchor_index(dyn_node_idx));
        let render_parent = MountedParent::component_root(mount, anchor_index, dyn_node_idx);
        self.runtime.mounts.borrow_mut()[root_mount.0].render_parent = Some(render_parent);
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
    /// Invariant: slot-writing paths and component root replacement paths update render parents
    /// before commit. Commit only swaps the vnode and compacts pending fragment storage into the
    /// committed layout.
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
        render_parent: MountedParent,
    ) {
        self.runtime.mounts.borrow_mut()[new_root_mount.0].render_parent = Some(render_parent);

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
        let text = MountedDynamicNodeSlot::Text(MountedElementId::new(ElementId::new(7)));
        let component = MountedDynamicNodeSlot::Component(ScopeId::new(3));

        assert_eq!(
            PackedMountedSlot::from_slot(MountedDynamicNodeSlot::Empty),
            PackedMountedSlot::empty()
        );
        assert_eq!(
            PackedMountedSlot::from_slot(text).text(),
            Some(MountedElementId::new(ElementId::new(7)))
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
/// mount identity.
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
