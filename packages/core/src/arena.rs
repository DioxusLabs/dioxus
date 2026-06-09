use crate::innerlude::ScopeOrder;
use crate::{ScopeId, virtual_dom::VirtualDom};
use slab::Slab;

/// An Element's unique identifier.
///
/// `ElementId` is a `usize` that is unique within one render target - but not
/// unique across targets or time. If a component is unmounted, then the
/// `ElementId` may be reused for a new component in that target.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ElementId(pub usize);

impl ElementId {
    /// The root element within a render target.
    pub const ROOT: Self = Self(0);
}

/// A renderer target's unique identifier.
///
/// Each render target has its own [`ElementId`] arena. This lets multiple
/// renderers share one logical [`VirtualDom`] while reusing renderer-local ids.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RenderTargetId(pub usize);

impl RenderTargetId {
    /// The root/default render target.
    pub const ROOT: Self = Self(0);
}

/// The kind of renderer backing a target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RenderTargetKind {
    /// A target backed by a real renderer.
    Real,

    /// A target that can keep logical tree state alive without materializing
    /// renderer nodes or running mount effects.
    Noop,
}

/// Renderer-local mounted state for one logical mount.
#[derive(Debug)]
pub(crate) struct MountedNodeState {
    /// The IDs for the roots of this template, used when moving or removing
    /// roots from the renderer.
    pub(crate) root_ids: Box<[ElementId]>,

    /// The element in the renderer that each dynamic attribute is mounted to.
    pub(crate) mounted_attributes: Box<[ElementId]>,

    /// For components: the `ScopeId` the component is mounted to.
    /// For other dynamic nodes: the renderer element id each dynamic node owns.
    pub(crate) mounted_dynamic_nodes: Box<[usize]>,
}

impl MountedNodeState {
    pub(crate) fn new(root_count: usize, attr_count: usize, dynamic_count: usize) -> Self {
        Self {
            root_ids: vec![ElementId(0); root_count].into(),
            mounted_attributes: vec![ElementId(0); attr_count].into(),
            mounted_dynamic_nodes: vec![usize::MAX; dynamic_count].into(),
        }
    }
}

/// Renderer-local state for a render target.
#[derive(Debug)]
pub(crate) struct RenderTargetState {
    pub(crate) kind: RenderTargetKind,
    pub(crate) elements: Slab<Option<ElementRef>>,
    pub(crate) mounts: Vec<Option<MountedNodeState>>,
}

impl RenderTargetState {
    pub(crate) fn new(kind: RenderTargetKind) -> Self {
        let mut elements = Slab::default();
        // The root element is always renderer-local element ID 0.
        elements.insert(None);

        Self {
            kind,
            elements,
            mounts: Vec::new(),
        }
    }

    pub(crate) fn create_mounted_node(
        &mut self,
        mount: MountId,
        root_count: usize,
        attr_count: usize,
        dynamic_count: usize,
    ) {
        if self.mounts.len() <= mount.0 {
            self.mounts.resize_with(mount.0 + 1, || None);
        }
        self.mounts[mount.0] = Some(MountedNodeState::new(root_count, attr_count, dynamic_count));
    }

    pub(crate) fn remove_mounted_node(&mut self, mount: MountId) {
        // Removal only happens for `mount` values just produced by the
        // mount-create path that allocated the slot, so the index is in
        // bounds by construction.
        debug_assert!(
            self.mounts.get(mount.0).is_some(),
            "remove_mounted_node called with unallocated mount",
        );
        self.mounts[mount.0].take();
    }
}

/// A mounted mount's unique identifier.
///
/// `MountId` is a `usize` that is unique across the current `VirtualDom` - but not unique across time. If a mount is
/// unmounted, then the `MountId` may be reused for a new mount.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MountId(pub(crate) usize);

impl Default for MountId {
    fn default() -> Self {
        Self::PLACEHOLDER
    }
}

impl MountId {
    pub(crate) const PLACEHOLDER: Self = Self(usize::MAX);

    #[allow(unused)]
    pub(crate) fn mounted(self) -> bool {
        self != Self::PLACEHOLDER
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ElementRef {
    // the pathway of the real element inside the template
    pub(crate) path: ElementPath,

    // The actual element
    pub(crate) mount: MountId,
}

#[derive(Clone, Copy, Debug)]
pub struct ElementPath {
    pub(crate) path: &'static [u8],
}

impl VirtualDom {
    pub(crate) fn current_render_target_id(&self) -> RenderTargetId {
        self.runtime.current_render_target_id()
    }

    pub(crate) fn mount_target_id(&self, mount: MountId) -> RenderTargetId {
        // Every caller has a live `mount` — either freshly allocated via
        // `next_element_for_mount` / mount creation, or the result of
        // `claim_mount` on a previously-mounted vnode. A PLACEHOLDER
        // here would indicate a stray ref the diff never produces.
        debug_assert!(mount.mounted(), "mount_target_id requires a live MountId");
        self.runtime
            .mounts
            .borrow()
            .get(mount.0)
            .map(|mount| mount.target_id)
            .unwrap_or(RenderTargetId::ROOT)
    }

    /// Number of template roots this `mount`'s mount was created with.
    /// Anchor lookups that walk a view's `template.roots()` may iterate
    /// beyond what the mount actually has — e.g. when the view was a clone
    /// whose template grew between renders — and the underlying
    /// `MountedNodeState::root_ids` would panic on out-of-range indexing.
    pub(crate) fn mounted_root_count(&self, mount: MountId) -> usize {
        debug_assert!(
            mount.mounted(),
            "mounted_root_count requires a live MountId"
        );
        let target_id = self.mount_target_id(mount);
        self.runtime
            .render_targets
            .borrow()
            .get(target_id.0)
            .and_then(|target| target.mounts.get(mount.0))
            .and_then(|mount| mount.as_ref())
            .map(|mount| mount.root_ids.len())
            .unwrap_or(0)
    }

    /// Number of dynamic-node slots this `mount`'s mount was created with.
    /// Same guard rail as [`Self::mounted_root_count`], but for
    /// `MountedNodeState::mounted_dynamic_nodes`.
    pub(crate) fn mounted_dyn_node_count(&self, mount: MountId) -> usize {
        debug_assert!(
            mount.mounted(),
            "mounted_dyn_node_count requires a live MountId"
        );
        let target_id = self.mount_target_id(mount);
        self.runtime
            .render_targets
            .borrow()
            .get(target_id.0)
            .and_then(|target| target.mounts.get(mount.0))
            .and_then(|mount| mount.as_ref())
            .map(|mount| mount.mounted_dynamic_nodes.len())
            .unwrap_or(0)
    }

    pub(crate) fn render_target_should_write(&self, target_id: RenderTargetId) -> bool {
        self.runtime
            .render_targets
            .borrow()
            .get(target_id.0)
            .is_some_and(|target| target.kind == RenderTargetKind::Real)
    }

    pub(crate) fn next_element_for_mount(&mut self, mount: MountId) -> ElementId {
        let target_id = self.mount_target_id(mount);
        self.next_element_in_target(target_id)
    }

    pub(crate) fn next_element_in_target(&mut self, target_id: RenderTargetId) -> ElementId {
        let mut targets = self.runtime.render_targets.borrow_mut();
        let target = targets
            .get_mut(target_id.0)
            .expect("render target should exist while allocating an element");
        ElementId(target.elements.insert(None))
    }

    pub(crate) fn reclaim_for_mount(&mut self, mount: MountId, el: ElementId) {
        let target_id = self.mount_target_id(mount);
        self.try_reclaim_in_target(target_id, el);
    }

    pub(crate) fn try_reclaim_in_target(
        &mut self,
        target_id: RenderTargetId,
        el: ElementId,
    ) -> bool {
        // Callers (`reclaim_for_mount` from diff/node.rs and the recursive
        // remove path) always pre-filter `ElementId::default()` and only fire
        // on real, allocated element ids. PLACEHOLDER ids are never reclaimed.
        debug_assert!(
            el.0 != 0 && el.0 != usize::MAX,
            "try_reclaim_in_target should never see ROOT or PLACEHOLDER ids",
        );

        let mut targets = self.runtime.render_targets.borrow_mut();
        let target = targets
            .get_mut(target_id.0)
            .expect("reclaim target must still be registered");
        target.elements.try_remove(el.0).is_some()
    }

    pub(crate) fn element_exists_for_mount(&self, mount: MountId, el: ElementId) -> bool {
        self.element_exists_in_target(self.mount_target_id(mount), el)
    }

    pub(crate) fn element_exists_in_target(
        &self,
        target_id: RenderTargetId,
        el: ElementId,
    ) -> bool {
        // Callers in diff/anchor.rs and diff/node.rs always pre-filter
        // `ElementId::default()` (ROOT) before calling, so we never see id 0.
        debug_assert!(el.0 != 0, "element_exists_in_target should never see ROOT");

        self.runtime
            .render_targets
            .borrow()
            .get(target_id.0)
            .is_some_and(|target| target.elements.get(el.0).is_some())
    }

    // Drop a scope without dropping its children
    //
    // Note: This will not remove any ids from the arena
    pub(crate) fn drop_scope(&mut self, id: ScopeId) {
        let stale_dirty_scopes: Vec<_> = self
            .dirty_scopes
            .iter()
            .filter_map(|order| {
                (order.id == id || self.runtime.is_descendant_of(order.id, id)).then_some(*order)
            })
            .collect();

        let stale_dirty_tasks: Vec<_> = self
            .runtime
            .dirty_tasks
            .borrow()
            .iter()
            .filter_map(|tasks| {
                (tasks.order.id == id || self.runtime.is_descendant_of(tasks.order.id, id))
                    .then_some(tasks.order)
            })
            .collect();

        let stale_pending_effects: Vec<_> = self
            .runtime
            .pending_effects
            .borrow()
            .iter()
            .filter_map(|effect| {
                (effect.order.id == id || self.runtime.is_descendant_of(effect.order.id, id))
                    .then_some(effect.order)
            })
            .collect();

        let scope = self.scopes.remove(id.0);
        let context = scope.state();
        let height = context.height;

        self.dirty_scopes.remove(&ScopeOrder::new(height, id));
        for order in stale_dirty_scopes {
            self.dirty_scopes.remove(&order);
        }
        {
            let mut dirty_tasks = self.runtime.dirty_tasks.borrow_mut();
            for order in stale_dirty_tasks {
                dirty_tasks.remove(&order);
            }
        }
        {
            let mut pending_effects = self.runtime.pending_effects.borrow_mut();
            for order in stale_pending_effects {
                pending_effects.remove(&order);
            }
        }

        // If this scope was a suspense boundary, remove it from the resolved scopes
        self.resolved_scopes.retain(|s| s != &id);
    }
}

impl ElementPath {
    pub(crate) fn is_descendant(&self, small: &[u8]) -> bool {
        small.len() <= self.path.len() && small == &self.path[..small.len()]
    }
}

#[test]
fn is_descendant() {
    let event_path = ElementPath {
        path: &[1, 2, 3, 4, 5],
    };

    assert!(event_path.is_descendant(&[1, 2, 3, 4, 5]));
    assert!(event_path.is_descendant(&[1, 2, 3, 4]));
    assert!(event_path.is_descendant(&[1, 2, 3]));
    assert!(event_path.is_descendant(&[1, 2]));
    assert!(event_path.is_descendant(&[1]));

    assert!(!event_path.is_descendant(&[1, 2, 3, 4, 5, 6]));
    assert!(!event_path.is_descendant(&[2, 3, 4]));
}

impl PartialEq<&[u8]> for ElementPath {
    fn eq(&self, other: &&[u8]) -> bool {
        self.path.eq(*other)
    }
}
