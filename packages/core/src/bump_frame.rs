use crate::nodes::RenderReturn;
use bumpalo::Bump;
use std::cell::{Cell, UnsafeCell};

pub(crate) struct BumpFrame {
    pub bump: UnsafeCell<Bump>,
    pub node: Cell<*const RenderReturn<'static>>,
}

impl BumpFrame {
    pub(crate) fn new(capacity: usize) -> Self {
        let bump = Bump::with_capacity(capacity);
        Self {
            bump: UnsafeCell::new(bump),
            node: Cell::new(std::ptr::null()),
        }
    }

    /// Creates a new lifetime out of thin air
    pub(crate) unsafe fn try_load_node<'b>(&self) -> Option<&'b RenderReturn<'b>> {
        let node = self.node.get();

        if node.is_null() {
            return None;
        }

        unsafe { std::mem::transmute(&*node) }
    }

    pub(crate) fn bump(&self) -> &Bump {
        unsafe { &*self.bump.get() }
    }

    #[allow(clippy::mut_from_ref)]
    pub(crate) unsafe fn bump_mut(&self) -> &mut Bump {
        unsafe { &mut *self.bump.get() }
    }
}
