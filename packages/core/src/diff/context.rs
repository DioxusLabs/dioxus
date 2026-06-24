use crate::{VirtualDom, WriteMutations, innerlude::MountId, nodes::VNode};

/// State required for diffing operations.
///
/// Invariant: one `DiffState` owns the active mutable access to the `VirtualDom` and the optional
/// renderer writer. `context`, when present, describes an active same-template vnode frame whose
/// committed mount table entry still points at the old vnode until the frame commits.
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

    /// Reborrow this state for a mount, disabling renderer writes if the mount is hidden.
    ///
    /// Invariant: disabling writes suppresses renderer mutations only; mount and component state
    /// still diff normally so hidden suspense branches remain current.
    pub(crate) fn reborrow_for_mount(
        &mut self,
        mount: MountId,
    ) -> DiffState<'_, 'ctx, '_, 'mutation> {
        let write = self.dom.mount_should_render(mount) && self.to.is_some();
        DiffState {
            dom: &mut *self.dom,
            to: self.to.as_deref_mut().filter(|_| write),
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

    /// Create replacement content in an empty dynamic slot, restore the old
    /// slot while removing the previous live node, then commit the new slot.
    ///
    /// Invariant: the slot is empty only during `create_new`. After the method returns, the slot is
    /// restored to the new value after old-node removal observes the old slot.
    pub(crate) fn replace_live_mounted_dynamic_node_slot<R>(
        &mut self,
        mount: MountId,
        dyn_node_idx: usize,
        create_new: impl FnOnce(&mut DiffState<'_, 'ctx, '_, 'mutation>) -> R,
        remove_old: impl FnOnce(&mut DiffState<'_, 'ctx, '_, 'mutation>),
    ) -> R {
        let old_slot = self
            .dom
            .mounted_dynamic_node_slot_snapshot(mount, dyn_node_idx);
        self.replace_mounted_dynamic_node_slot(mount, dyn_node_idx, create_new, |state| {
            state
                .dom
                .restore_mounted_dynamic_node_slot(mount, dyn_node_idx, old_slot);
            remove_old(state);
        })
    }

    pub(crate) fn replace_mounted_dynamic_node_slot<R>(
        &mut self,
        mount: MountId,
        dyn_node_idx: usize,
        create_new: impl FnOnce(&mut DiffState<'_, 'ctx, '_, 'mutation>) -> R,
        before_commit_new: impl FnOnce(&mut DiffState<'_, 'ctx, '_, 'mutation>),
    ) -> R {
        self.dom
            .clear_mounted_dynamic_node_slot(mount, dyn_node_idx);

        let result = create_new(self);
        let new_slot = self
            .dom
            .mounted_dynamic_node_slot_snapshot(mount, dyn_node_idx);

        before_commit_new(self);

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

impl<'a> DiffFrame<'a> {
    pub(crate) fn new(mount: MountId, old: &'a VNode, new: &'a VNode) -> Self {
        Self { mount, old, new }
    }
}

/// Diff-local view of the active vnode and its parent while children are being
/// reconciled.
///
/// The committed mount table still points at the old vnode while a same-template frame is being
/// diffed, so placement resolution needs these temporary old/new pairs to reason about slots inside
/// the active vnode and sibling order in the active parent.
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
