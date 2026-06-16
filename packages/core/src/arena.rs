use crate::{ScopeId, Template, virtual_dom::VirtualDom};
use slab::Slab;
use std::collections::HashMap;
use std::num::NonZeroUsize;

/// An Element's unique identifier.
///
/// `ElementId` is a `usize` that is unique within one render target - but not
/// unique across targets or time. If a component is unmounted, then the
/// `ElementId` may be reused for a new component in that target.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementId(usize);

impl ElementId {
    /// The root element within a render target.
    pub const ROOT: Self = Self(0);

    pub(crate) const fn new(index: usize) -> Self {
        Self(index)
    }

    pub(crate) const fn index(self) -> usize {
        self.0
    }

    /// Create an element id from its raw renderer-local index.
    ///
    /// Renderers use this when translating platform event targets back into
    /// Dioxus element ids.
    pub const fn from_raw(index: usize) -> Self {
        Self(index)
    }

    /// Return this element id's raw renderer-local index.
    ///
    /// Renderers use this to store Dioxus element ids in their backing DOM or
    /// interpreter node maps.
    pub const fn raw(self) -> usize {
        self.0
    }
}

/// An allocated, non-root renderer element id.
///
/// This type is used inside mounted-state tables so absence is represented by
/// `Option<MountedElementId>` instead of sentinel `ElementId` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MountedElementId(NonZeroUsize);

impl MountedElementId {
    pub(crate) fn new_unchecked(id: ElementId) -> Self {
        debug_assert!(id != ElementId::ROOT);
        Self::from_index_unchecked(id.index())
    }

    pub(crate) fn from_index_unchecked(index: usize) -> Self {
        debug_assert_ne!(index, ElementId::ROOT.index());
        Self(NonZeroUsize::new(index).expect("mounted element id cannot be the root element"))
    }

    pub(crate) fn element_id(self) -> ElementId {
        ElementId::new(self.index())
    }

    pub(crate) fn index(self) -> usize {
        self.0.get()
    }
}

/// A renderer target's unique identifier.
///
/// Each render target has its own [`ElementId`] arena. This lets multiple
/// renderers share one logical [`VirtualDom`] while reusing renderer-local ids.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RenderTargetId(usize);

impl RenderTargetId {
    /// The root/default render target.
    pub const ROOT: Self = Self(0);

    pub(crate) const fn new(index: usize) -> Self {
        Self(index)
    }

    pub(crate) const fn index(self) -> usize {
        self.0
    }
}

/// The mounted target for one dynamic node slot.
///
/// For components this records the child scope and the mount that owns its
/// root nodes; for other dynamic nodes it records the renderer element the
/// node is mounted to. Both live on the VirtualDom-side [`Mount`], indexed by
/// dynamic-node index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MountedDynamicNodeSlot {
    Empty,
    Text(MountedElementId),
    Component(ScopeId),
    Fragment(usize),
}

/// Renderer-local state for a render target.
#[derive(Debug)]
pub(crate) struct RenderTargetState {
    pub(crate) elements: Slab<Option<MountRef>>,
    pub(crate) template_roots: HashMap<(Template, usize), MountedElementId>,
}

impl RenderTargetState {
    pub(crate) fn new() -> Self {
        let mut elements = Slab::default();
        // The root element is always renderer-local element ID 0.
        elements.insert(None);

        Self {
            elements,
            template_roots: HashMap::new(),
        }
    }

    pub(crate) fn reset_for_rebuild(&mut self) {
        self.elements.clear();
        // The root element is always renderer-local element ID 0.
        self.elements.insert(None);
        self.template_roots.clear();
    }
}

/// A live mount's unique identifier.
///
/// `MountId` is a `usize` that is unique across the current `VirtualDom` - but not unique across time. If a mount is
/// unmounted, then the `MountId` may be reused for a new mount.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MountId(pub(crate) usize);

#[derive(Debug, Clone, Copy)]
pub struct MountRef {
    // The mount that owns the renderer element.
    pub(crate) mount: MountId,
}

impl VirtualDom {
    pub(crate) fn current_render_target_id(&self) -> RenderTargetId {
        self.runtime.current_render_target_id()
    }

    pub(crate) fn next_element_in_target(&mut self, target_id: RenderTargetId) -> MountedElementId {
        let mut targets = self.runtime.render_targets.borrow_mut();
        let target = targets
            .get_mut(target_id.index())
            .expect("render target should exist while allocating an element");
        MountedElementId::new_unchecked(ElementId::new(target.elements.insert(None)))
    }

    pub(crate) fn cached_template_root(
        &self,
        target_id: RenderTargetId,
        template: Template,
        root_idx: usize,
    ) -> Option<MountedElementId> {
        self.runtime
            .render_targets
            .borrow()
            .get(target_id.index())
            .and_then(|target| target.template_roots.get(&(template, root_idx)).copied())
    }

    pub(crate) fn allocate_template_root(
        &mut self,
        target_id: RenderTargetId,
        template: Template,
        root_idx: usize,
    ) -> MountedElementId {
        let mut targets = self.runtime.render_targets.borrow_mut();
        let target = targets
            .get_mut(target_id.index())
            .expect("render target should exist while allocating a template root");
        let id = MountedElementId::new_unchecked(ElementId::new(target.elements.insert(None)));
        target.template_roots.insert((template, root_idx), id);
        id
    }

    pub(crate) fn reset_render_targets_for_rebuild(&mut self) {
        for (_, target) in self.runtime.render_targets.borrow_mut().iter_mut() {
            target.reset_for_rebuild();
        }
    }

    pub(crate) fn set_element_ref_for_mount(&self, mount: MountId, el: MountedElementId) {
        let target_id = self.mount_target_id(mount);
        let mut targets = self.runtime.render_targets.borrow_mut();
        let target = targets
            .get_mut(target_id.index())
            .expect("render target should exist while assigning an element ref");
        let element = target
            .elements
            .get_mut(el.index())
            .expect("element should exist while assigning an element ref");
        *element = Some(MountRef { mount });
    }

    pub(crate) fn reclaim_for_mount(&mut self, mount: MountId, el: MountedElementId) {
        let target_id = self.mount_target_id(mount);
        self.try_reclaim_in_target(target_id, el);
    }

    pub(crate) fn try_reclaim_in_target(
        &mut self,
        target_id: RenderTargetId,
        el: MountedElementId,
    ) -> bool {
        let mut targets = self.runtime.render_targets.borrow_mut();
        let target = targets
            .get_mut(target_id.index())
            .expect("reclaim target must still be registered");
        target.elements.try_remove(el.index()).is_some()
    }

    pub(crate) fn element_exists_for_mount(&self, mount: MountId, el: MountedElementId) -> bool {
        self.element_exists_in_target(self.mount_target_id(mount), el)
    }

    pub(crate) fn element_exists_in_target(
        &self,
        target_id: RenderTargetId,
        el: MountedElementId,
    ) -> bool {
        self.runtime
            .render_targets
            .borrow()
            .get(target_id.index())
            .is_some_and(|target| target.elements.get(el.index()).is_some())
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

        self.mark_clean(id);
        let _scope = self.scopes.remove(id.index());

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
