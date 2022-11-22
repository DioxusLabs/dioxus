use crate::factory::RenderReturn;
use bumpalo::Bump;
use std::cell::Cell;

pub struct BumpFrame {
    pub bump: Bump,
    pub node: Cell<*mut RenderReturn<'static>>,
}

impl BumpFrame {
    pub fn new(capacity: usize) -> Self {
        let bump = Bump::with_capacity(capacity);
        Self {
            bump,
            node: Cell::new(std::ptr::null_mut()),
        }
    }

    pub fn reset(&mut self) {
        self.bump.reset();
        self.node.set(std::ptr::null_mut());
    }

    /// Creates a new lifetime out of thin air
    pub unsafe fn load_node<'b>(&self) -> &'b RenderReturn<'b> {
        unsafe { std::mem::transmute(&*self.node.get()) }
    }
}
