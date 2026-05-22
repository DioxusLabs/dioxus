use crate::{
    RenderTargetId, VNode,
    innerlude::{ElementRef, MountId},
};

/// Opaque identity for one mounted virtual-DOM fiber.
///
/// A `FiberId` is stable for the lifetime of a mounted fiber, but it must not
/// be interpreted as an arena index. It is exposed so renderers and diagnostics
/// can correlate cooperative scheduler checkpoints without depending on
/// internal mount bookkeeping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FiberId(pub(crate) u64);

/// Whether a fiber is allowed to write renderer mutations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FiberMode {
    Foreground,
    Background,
}

/// Persistent render identity for one mounted `VNode`.
///
/// A fiber owns the renderer ids and dynamic child bindings for an rsx block.
/// `node` is the committed view used after diffing for event dispatch, tree
/// inspection, and the next render pass.
#[derive(Debug)]
pub(crate) struct Fiber {
    /// Stable opaque identity for diagnostics and cooperative scheduling.
    pub(crate) id: FiberId,

    /// The physical parent used for renderer placement and anchors.
    pub(crate) render_parent: Option<ElementRef>,

    /// The logical parent used for context tree event bubbling.
    pub(crate) logical_parent: Option<ElementRef>,

    /// The render target this fiber is materialized into.
    pub(crate) target_id: RenderTargetId,

    /// The committed view used for events and mounted tree inspection.
    pub(crate) node: VNode,

    /// Suspense can keep a primary branch alive while its fallback is visible.
    /// Background fibers may update their virtual tree, but they must not write
    /// renderer mutations until they are promoted back to the foreground.
    pub(crate) mode: FiberMode,
}

impl Fiber {
    pub(crate) fn new(
        id: FiberId,
        node: VNode,
        render_parent: Option<ElementRef>,
        logical_parent: Option<ElementRef>,
        target_id: RenderTargetId,
    ) -> Self {
        Self {
            id,
            render_parent,
            logical_parent,
            target_id,
            node,
            mode: FiberMode::Foreground,
        }
    }
}

/// A retained suspense branch.
///
/// Suspense keeps the hidden primary branch alive while the fallback branch is
/// visible. The root `VNode` is still the render output we diff, but the branch
/// also records the root fiber identity so the boundary state is explicitly tied
/// to retained fiber ownership instead of being just a parked vnode.
#[derive(Clone, Debug)]
pub(crate) struct SuspenseBranch {
    root: VNode,
    root_fiber: MountId,
}

impl SuspenseBranch {
    pub(crate) fn new(root: VNode) -> Self {
        let root_fiber = root.mount.get();
        debug_assert!(
            root_fiber.mounted(),
            "suspense branches must have a mounted root fiber"
        );
        Self { root, root_fiber }
    }

    pub(crate) fn root(&self) -> VNode {
        self.root.clone()
    }

    pub(crate) fn root_fiber(&self) -> MountId {
        self.root_fiber
    }

    pub(crate) fn into_root(self) -> VNode {
        self.root
    }
}
