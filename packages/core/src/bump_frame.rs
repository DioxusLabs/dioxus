use crate::factory::RenderReturn;
use bumpalo::Bump;
use std::cell::Cell;

pub(crate) struct BumpFrame {
    pub bump: Bump,
    pub node: Cell<*const RenderReturn<'static>>,
}

impl BumpFrame {
    pub fn new(capacity: usize) -> Self {
        let bump = Bump::with_capacity(capacity);
        Self {
            bump,
            node: Cell::new(std::ptr::null()),
        }
    }

    /// Creates a new lifetime out of thin air
    pub unsafe fn try_load_node<'b>(&self) -> Option<&'b RenderReturn<'b>> {
        let node = self.node.get();

        if node.is_null() {
            return None;
        }

        unsafe { std::mem::transmute(&*node) }
    }
}
