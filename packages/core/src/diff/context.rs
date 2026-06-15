use crate::{
    VirtualDom, WriteMutations, innerlude::MountId, mutations::reborrow_writer, nodes::VNode,
};

/// State required for diffing operations
pub(crate) struct DiffState<'dom, 'ctx, 'writer> {
    pub(crate) dom: &'dom mut VirtualDom,
    pub(crate) to: Option<&'writer mut (dyn WriteMutations + 'writer)>,
    pub(crate) context: Option<DiffContext<'ctx>>,
}

impl<'dom, 'ctx, 'writer> DiffState<'dom, 'ctx, 'writer> {
    pub(crate) fn new(
        dom: &'dom mut VirtualDom,
        to: Option<&'writer mut (dyn WriteMutations + 'writer)>,
    ) -> Self {
        Self::new_with_context(dom, to, None)
    }

    pub(crate) fn new_with_context(
        dom: &'dom mut VirtualDom,
        to: Option<&'writer mut (dyn WriteMutations + 'writer)>,
        context: Option<DiffContext<'ctx>>,
    ) -> Self {
        Self { dom, to, context }
    }

    pub(crate) fn reborrow_with_writes(&mut self, write: bool) -> DiffState<'_, 'ctx, '_> {
        DiffState {
            dom: &mut *self.dom,
            to: if write {
                reborrow_writer(&mut self.to)
            } else {
                None
            },
            context: self.context,
        }
    }

    pub(crate) fn context(&self) -> Option<DiffContext<'ctx>> {
        self.context
    }

    /// Create replacement content in an empty dynamic slot, then optionally
    /// restore the old slot while removing the previous live node.
    pub(crate) fn with_mounted_dynamic_node_slot_replaced<R>(
        &mut self,
        mount: MountId,
        dyn_node_idx: usize,
        restore_old_slot_for_removal: bool,
        create_new: impl FnOnce(&mut DiffState<'_, 'ctx, '_>) -> R,
        remove_old: impl FnOnce(&mut DiffState<'_, 'ctx, '_>),
    ) -> R {
        let old_slot = self.dom.get_mounted_dynamic_node_slot(mount, dyn_node_idx);
        self.dom
            .clear_mounted_dynamic_node_slot(mount, dyn_node_idx);

        let result = create_new(self);
        let new_slot = self.dom.get_mounted_dynamic_node_slot(mount, dyn_node_idx);

        if restore_old_slot_for_removal {
            self.dom
                .set_mounted_dynamic_node_slot(mount, dyn_node_idx, old_slot);
            remove_old(self);
        }

        self.dom
            .set_mounted_dynamic_node_slot(mount, dyn_node_idx, new_slot);
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

#[derive(Clone, Copy, Debug)]
pub(crate) struct DiffFrame<'a> {
    pub(crate) mount: MountId,
    pub(crate) old: &'a VNode,
    pub(crate) new: &'a VNode,
}

/// Diff-local view of the active vnode and its parent while children are being
/// reconciled.
///
/// The committed mount still points at the old vnode until a vnode finishes
/// diffing, so anchor resolution needs these temporary old/new pairs to reason
/// about slots inside the active vnode and sibling order in the active parent.
#[derive(Clone, Copy, Debug)]
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
