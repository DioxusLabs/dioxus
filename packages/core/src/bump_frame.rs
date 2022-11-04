use std::cell::Cell;

use bumpalo::Bump;

use crate::factory::RenderReturn;

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
}
