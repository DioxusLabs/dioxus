use crate::{VirtualDom, WriteMutations, innerlude::MountId, nodes::VNode};

/// State required for diffing operations.
///
/// Invariant: one `DiffState` owns the active mutable access to the `VirtualDom` and the optional
/// renderer writer. `context` describes the vnode frame currently being diffed. Mounts whose
/// committed position is stale (moved or replaced by the active diff) are tracked on the
/// `Runtime`, so placement scans can consult them in O(1).
pub(crate) struct DiffState<'dom, 'ctx, 'writer, 'mutation> {
    pub(crate) dom: &'dom mut VirtualDom,
    pub(crate) to: Option<&'writer mut (dyn WriteMutations + 'mutation)>,
    pub(crate) context: Option<DiffContext<'ctx>>,
}

impl<'dom, 'ctx, 'writer, 'mutation> DiffState<'dom, 'ctx, 'writer, 'mutation> {
    pub(crate) fn new(
        dom: &'dom mut VirtualDom,
        to: Option<&'writer mut (dyn WriteMutations + 'mutation)>,
    ) -> Self {
        Self::new_with_context(dom, to, None)
    }

    pub(crate) fn new_with_context(
        dom: &'dom mut VirtualDom,
        to: Option<&'writer mut (dyn WriteMutations + 'mutation)>,
        context: Option<DiffContext<'ctx>>,
    ) -> Self {
        Self { dom, to, context }
    }

    /// Reborrow this state while optionally disabling renderer writes.
    ///
    /// Invariant: disabling writes suppresses renderer mutations only; mount and component state
    /// still diff normally so hidden suspense branches remain current.
    pub(crate) fn reborrow_with_writes(
        &mut self,
        write: bool,
    ) -> DiffState<'_, 'ctx, '_, 'mutation> {
        DiffState {
            dom: &mut *self.dom,
            to: if write { self.to.as_deref_mut() } else { None },
            context: self.context,
        }
    }

    pub(crate) fn context(&self) -> Option<DiffContext<'ctx>> {
        self.context
    }

    /// Whether a renderer writer is attached for the current diff.
    ///
    /// A no-op writer still counts as attached: it absorbs the same mutation
    /// stream as a real renderer so the diff keeps one control flow. Only a
    /// hidden/suppressed diff (no writer at all) skips renderer placement.
    pub(crate) fn has_writer(&mut self) -> bool {
        self.to.is_some()
    }

    /// Create replacement content in an empty dynamic slot, then optionally
    /// restore the old slot while removing the previous live node.
    ///
    /// Invariant: the slot is empty only during `create_new`. After the method returns, the slot is
    /// restored to the new value even if old-node removal needed the old slot temporarily visible.
    pub(crate) fn with_mounted_dynamic_node_slot_replaced<R>(
        &mut self,
        mount: MountId,
        dyn_node_idx: usize,
        restore_old_slot_for_removal: bool,
        create_new: impl FnOnce(&mut DiffState<'_, 'ctx, '_, 'mutation>) -> R,
        remove_old: impl FnOnce(&mut DiffState<'_, 'ctx, '_, 'mutation>),
    ) -> R {
        let old_slot = self
            .dom
            .mounted_dynamic_node_slot_snapshot(mount, dyn_node_idx);
        self.dom
            .clear_mounted_dynamic_node_slot(mount, dyn_node_idx);

        let result = create_new(self);
        let new_slot = self
            .dom
            .mounted_dynamic_node_slot_snapshot(mount, dyn_node_idx);

        if restore_old_slot_for_removal {
            self.dom
                .restore_mounted_dynamic_node_slot(mount, dyn_node_idx, old_slot);
            remove_old(self);
        }

        self.dom
            .restore_mounted_dynamic_node_slot(mount, dyn_node_idx, new_slot);
        result
    }

    pub(crate) fn enter_context(&mut self, mount: MountId, old: &'ctx VNode, new: &'ctx VNode) {
        let context = self.context.map_or_else(
            || DiffContext::new(mount, old, new),
            |context| context.enter(mount, old, new),
        );
        self.context = Some(context);
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DiffFrame<'a> {
    pub(crate) mount: MountId,
    pub(crate) old: &'a VNode,
    pub(crate) new: &'a VNode,
}

/// Diff-local view of the active vnode and its parent while children are being
/// reconciled.
///
/// The committed mount still points at the old vnode until a vnode finishes
/// diffing, so placement resolution needs these temporary old/new pairs to reason
/// about slots inside the active vnode and sibling order in the active parent.
#[derive(Clone, Copy)]
pub(crate) struct DiffContext<'a> {
    current: DiffFrame<'a>,
    parent: Option<DiffFrame<'a>>,
}

impl<'a> DiffContext<'a> {
    pub(crate) fn new(mount: MountId, old: &'a VNode, new: &'a VNode) -> Self {
        Self {
            current: DiffFrame { mount, old, new },
            parent: None,
        }
    }

    pub(crate) fn enter(self, mount: MountId, old: &'a VNode, new: &'a VNode) -> Self {
        Self {
            current: DiffFrame { mount, old, new },
            parent: Some(self.current),
        }
    }

    pub(crate) fn for_mount(self, mount: MountId) -> Option<DiffFrame<'a>> {
        if self.current.mount == mount {
            Some(self.current)
        } else {
            self.parent.filter(|frame| frame.mount == mount)
        }
    }
}

impl<'a> DiffFrame<'a> {
    pub(crate) fn new(mount: MountId, old: &'a VNode, new: &'a VNode) -> Self {
        Self { mount, old, new }
    }
}
